using System.IO;
using System.Linq;
using System.Text.Json;
using System.Xml.Linq;
using Xunit;

namespace Lingara.Embed.Server.Tests;

public class PackageShapeTests
{
    private static readonly string Project = Repo.Path("dotnet", "src", "Lingara.Embed.Server");

    /// <summary>
    /// 1.10.26af AC31: the package has no <c>PackageReference</c> (in its
    /// manifest, and in the restored graph, so nothing inherited from
    /// <c>Directory.Build.props</c> adds one), targets only <c>net8.0</c>, and
    /// exposes no type from <c>Lingara.Embed</c>, the player half.
    /// </summary>
    [Fact]
    public void HasNoDependencies()
    {
        var manifest = XDocument.Load(Path.Combine(Project, "Lingara.Embed.Server.csproj"));
        Assert.Empty(manifest.Descendants("PackageReference"));
        Assert.Empty(manifest.Descendants("ProjectReference"));
        Assert.Equal("net8.0", manifest.Descendants("TargetFramework").Single().Value);
        Assert.Empty(manifest.Descendants("TargetFrameworks"));

        // The restored graph: the SDK's own build-time ILLink tasks (IsAotCompatible)
        // are the only package, auto-referenced and private, so never in the nuspec.
        using var assets = JsonDocument.Parse(File.ReadAllText(Path.Combine(Project, "obj", "project.assets.json")));
        Assert.Equal(new[] { "net8.0" }, assets.RootElement.GetProperty("targets").EnumerateObject().Select(t => t.Name));
        var restored = assets.RootElement.GetProperty("project").GetProperty("frameworks").GetProperty("net8.0");
        foreach (var dependency in restored.TryGetProperty("dependencies", out var d) ? d.EnumerateObject() : default)
        {
            Assert.True(dependency.Value.TryGetProperty("autoReferenced", out var auto) && auto.GetBoolean(), dependency.Name);
            Assert.Equal("All", dependency.Value.GetProperty("suppressParent").GetString());
        }

        var assembly = typeof(EmbedServer).Assembly;
        Assert.DoesNotContain(assembly.GetReferencedAssemblies(), a => a.Name == "Lingara.Embed");
        var surface = assembly.GetExportedTypes()
            .SelectMany(t => t.GetMembers().Select(m => m.DeclaringType).Append(t.BaseType))
            .Where(t => t is not null);
        Assert.DoesNotContain(surface, t => t!.Assembly.GetName().Name == "Lingara.Embed");
    }
}
