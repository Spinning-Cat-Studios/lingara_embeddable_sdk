<?php

declare(strict_types=1);

namespace Lingara\Embed\ConformanceEs;

use Lingara\Embed\EmbedServer;
use Lingara\Embed\TokenHandoff;
use Lingara\Exception\ApiException;
use Lingara\Exception\MaintenanceException;
use Lingara\Exception\OAuthException;
use Lingara\Exception\TransportException;
use Lingara\MintedToken;

/** What a case's `kit:` jobs left behind. */
final class Outcome
{
    public ?MintedToken $minted = null;
    public ?string $handoff = null;
    /** @var array{status: int, code: string}|null */
    public ?array $error = null;
    public bool $render = false;
}

/** A case's `kit:` jobs, in order, through EmbedServer and TokenHandoff. */
final class Jobs
{
    /**
     * Every job in order; a raised error is recorded and the next job runs.
     * After each, the receiver sees the directory's rows.
     *
     * @param array<mixed> $jobs
     */
    public static function run(EmbedServer $server, array $jobs, Recording $directory, SharedState $state): Outcome
    {
        $out = new Outcome();
        foreach ($jobs as $job) {
            try {
                self::one($server, is_array($job) ? $job : [], $out);
            } catch (\Throwable $e) {
                $out->error = self::raised($e);
            }
            Receiver::syncDirectory($state, $directory->snapshot());
        }
        return $out;
    }

    /** @param array<mixed> $job */
    private static function one(EmbedServer $server, array $job, Outcome $out): void
    {
        if (is_array($job['mint'] ?? null)) {
            $out->minted = self::mint($server, $job['mint']);
        } elseif (is_array($job['delete'] ?? null)) {
            $server->deletePlayer(self::text($job['delete'], 'player_ref'));
        } elseif (array_key_exists('handoff', $job)) {
            $out->handoff = $out->minted === null ? null : TokenHandoff::body($out->minted);
        } else {
            $out->render = true;
        }
    }

    /** @param array<mixed> $m */
    private static function mint(EmbedServer $server, array $m): MintedToken
    {
        $scopes = is_array($m['scopes'] ?? null) ? array_values(array_filter($m['scopes'], 'is_string')) : null;
        $origin = is_string($m['origin'] ?? null) ? $m['origin'] : null;
        return $server->mintToken(self::text($m, 'player_ref'), $scopes, $origin);
    }

    /** @param array<mixed> $v */
    private static function text(array $v, string $key): string
    {
        return is_string($v[$key] ?? null) ? $v[$key] : '';
    }

    /**
     * `{status, code}`: an API error's, an OAuth error's `error`, `maintenance`.
     *
     * @return array{status: int, code: string}
     */
    private static function raised(\Throwable $e): array
    {
        return match (true) {
            $e instanceof ApiException => ['status' => $e->status(), 'code' => $e->errorCode()],
            $e instanceof OAuthException => ['status' => $e->status(), 'code' => $e->error()],
            $e instanceof MaintenanceException => ['status' => 503, 'code' => 'maintenance'],
            $e instanceof TransportException => ['status' => 0, 'code' => $e->kind()->value],
            default => ['status' => 0, 'code' => $e::class . ': ' . $e->getMessage()],
        };
    }
}
