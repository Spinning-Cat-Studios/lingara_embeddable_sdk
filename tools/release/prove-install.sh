#!/bin/sh
# prove-install.sh <id> <tag> [--deadline 15m] (ADR 1.10.26ag D7; the clients' 29.9.26v D8 shape)
#
# Runs the install line the site shows (`release-manifest install-line`, which
# reads the entry's snippets directory) against the registry it names, in an
# empty scratch project outside the checkout, so nothing resolves from the
# working tree. Registries propagate at different speeds, so it retries every
# 60 seconds, or at the deadline if that is sooner, then exits 1 with
# `install proof failed: <id> <line>`.
#
# Every attempt starts cold: a fresh scratch directory holding every tool's
# cache, because each tool remembers a failed lookup. A store (godot, unreal)
# has no line to prove, and neither has an unknown id: both are exit 2, before
# any attempt.
set -eu

usage() { echo "usage: prove-install.sh <id> <tag> [--deadline <N>m|<N>s]" >&2; exit 2; }

id=${1:-}
tag=${2:-}
[ -n "$id" ] && [ -n "$tag" ] || usage
shift 2
deadline=15m
while [ $# -gt 0 ]; do
    case "$1" in
        --deadline) [ $# -ge 2 ] || usage; deadline=$2; shift 2 ;;
        *) usage ;;
    esac
done
case "$deadline" in
    *m) seconds=$(( ${deadline%m} * 60 )) ;;
    *s) seconds=${deadline%s} ;;
    *) seconds=$deadline ;;
esac

case "$id" in
    core | bevy | dotnet | dotnetserver | typescript | kotlin | kotlinserver | java | javaserver | \
        nativesdesktop | nativesandroid | go | ruby | php) ;;
    godot | unreal) echo "prove-install.sh: $id is a store; there is no install line to prove (Manual steps 7-8)" >&2; exit 2 ;;
    *) echo "prove-install.sh: no proof for $id" >&2; exit 2 ;;
esac

checkout=$(pwd)
here=$(cd "$(dirname "$0")" && pwd)
semver=${tag#v}
command -v release-manifest >/dev/null 2>&1 ||
    { echo "prove-install.sh: release-manifest is not on PATH (make release-manifest-install)" >&2; exit 2; }
line=$(release-manifest install-line "$id" "$tag")

# The install line spliced into a fresh project of each build tool.
pom() {
    { echo '<project xmlns="http://maven.apache.org/POM/4.0.0">'
      echo '  <modelVersion>4.0.0</modelVersion>'
      echo '  <groupId>proof</groupId><artifactId>proof</artifactId><version>0</version>'
      echo '  <dependencies>'; echo "$line"; echo '  </dependencies>'
      echo '</project>'; } > pom.xml
}

csproj() {
    file=$(ls ./*.csproj) &&
    sed "s#</Project>#<ItemGroup>$line</ItemGroup></Project>#" "$file" > "$file.new" && mv "$file.new" "$file"
}

# Only dependency metadata is cold: the runner's Gradle distribution is
# copied in rather than downloaded again.
gradle_deps() {
    : > settings.gradle.kts &&
    printf 'plugins { java }\nrepositories { mavenCentral() }\ndependencies {\n%s\n}\n' "$line" > build.gradle.kts &&
    mkdir -p "$1/gradle/wrapper" &&
    { [ ! -d "$HOME/.gradle/wrapper/dists" ] || cp -R "$HOME/.gradle/wrapper/dists" "$1/gradle/wrapper/"; } &&
    GRADLE_USER_HOME="$1/gradle" "$checkout/gradlew" -p "$1" dependencies \
        --configuration runtimeClasspath --refresh-dependencies > "$1/deps.txt" 2>&1 &&
    ! grep -q FAILED "$1/deps.txt"
}

# The fetched file holds every rid of release.toml's package row.
holds_every_rid() {
    entries=$(unzip -Z1 "$2") || return 1
    for rid in $(bash "$here/release-toml.sh" package-rids "$1"); do
        for path in $(bash "$here/release-toml.sh" package "$1" "$rid"); do
            echo "$entries" | grep -q "^$path." || { echo "prove-install.sh: $2 has nothing under $path" >&2; return 1; }
        done
    done
}

# A natives artefact is proved by its own coordinate; the shared line names
# the kit and its natives.
natives() {
    artifact=lingara-embed-natives-$1 ext=$2
    mvn -q -U -Dmaven.repo.local="$scratch/m2" dependency:get -Dtransitive=false \
        -Dartifact="com.getlingara:$artifact:$semver${ext:+:$ext}" &&
    holds_every_rid "$3" "$scratch/m2/com/getlingara/$artifact/$semver/$artifact-$semver.${ext:-jar}"
}

# One attempt in scratch directory $1. It runs as an `if` condition, where
# `set -e` does not apply, so every step is chained with `&&`. The line itself
# runs under `sh -e`, so a two-line snippet stops at its first failure.
attempt() {
    scratch=$1
    cd "$scratch" || return 1
    case "$id" in
        core | bevy)
            cargo new --quiet proof && cd proof &&
            CARGO_HOME="$scratch/cargo" sh -ec "$line" &&
            CARGO_HOME="$scratch/cargo" cargo fetch ;;
        dotnet)
            dotnet new classlib -o proof >/dev/null && cd proof && csproj &&
            NUGET_PACKAGES="$scratch/nuget" NUGET_HTTP_CACHE_PATH="$scratch/nuget-http" dotnet restore >/dev/null &&
            holds_restored "$scratch/nuget/lingara.embed/$(echo "$semver" | tr '[:upper:]' '[:lower:]')" ;;
        dotnetserver)
            dotnet new classlib -f net8.0 -o proof >/dev/null && cd proof && csproj &&
            NUGET_PACKAGES="$scratch/nuget" NUGET_HTTP_CACHE_PATH="$scratch/nuget-http" dotnet build >/dev/null ;;
        typescript)
            npm init -y >/dev/null &&
            npm_config_cache="$scratch/npm" npm_config_prefer_online=true sh -ec "$line" &&
            node -e "import('@lingara/embed')" &&
            node -e "import('@lingara/embed/server')" ;;
        kotlin | kotlinserver) gradle_deps "$scratch" ;;
        java | javaserver) pom && mvn -q -U -Dmaven.repo.local="$scratch/m2" dependency:resolve ;;
        nativesdesktop) natives desktop "" nativesdesktop ;;
        nativesandroid) natives android aar nativesandroid ;;
        go)
            go mod init proof >/dev/null 2>&1 &&
            GOMODCACHE="$scratch/gomod" GOFLAGS=-modcacherw sh -ec "$line" ;;
        ruby)
            GEM_HOME="$scratch/gems" GEM_PATH="$scratch/gems" sh -ec "$line" &&
            GEM_HOME="$scratch/gems" GEM_PATH="$scratch/gems" ruby -e 'require "lingara/embed"' ;;
        php)
            composer init -n --name proof/proof >/dev/null &&
            COMPOSER_CACHE_DIR="$scratch/composer" sh -ec "$line" ;;
    esac
}

# All five runtimes/<rid>/native/ directories in the restored Lingara.Embed.
holds_restored() {
    for rid in $(bash "$here/release-toml.sh" package-rids dotnet); do
        path=$(bash "$here/release-toml.sh" package dotnet "$rid")
        [ -n "$(ls -A "$1/$path" 2>/dev/null)" ] || { echo "prove-install.sh: the restored package has no $path" >&2; return 1; }
    done
}

end=$(( $(date +%s) + seconds ))
while :; do
    scratch=$(mktemp -d "${TMPDIR:-/tmp}/prove-install.XXXXXX")
    if (attempt "$scratch"); then
        rm -rf "$scratch"
        echo "✓ install proof: $id $line"
        exit 0
    fi
    rm -rf "$scratch"
    now=$(date +%s)
    if [ "$now" -ge "$end" ]; then
        echo "install proof failed: $id $line" >&2
        exit 1
    fi
    wait=$(( end - now )); [ "$wait" -le 60 ] || wait=60
    echo "prove-install.sh: $id not installable yet; retrying in ${wait}s" >&2
    sleep "$wait"
done
