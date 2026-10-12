// K3's C ABI as an External module (ADR 1.10.26ac D3): the header, the
// engine-free C++ wrapper, and one prebuilt library per platform under
// lib/<Platform>/, staged from dist/ffi/<rid>/ by `make unreal-stage`.
//
// Dynamic on Win64, Mac, Linux and Android: K3 links its DLL with
// +crt-static so no memory crosses the ABI, and its static .lib would clash
// with the engine's /MD link. Static on iOS, where a loose dylib is not
// allowed.
using System.IO;
using UnrealBuildTool;

public class LingaraEmbedLibrary : ModuleRules
{
	public LingaraEmbedLibrary(ReadOnlyTargetRules Target) : base(Target)
	{
		Type = ModuleType.External;
		PublicSystemIncludePaths.Add(Path.Combine(ModuleDirectory, "include"));
		string Lib = Path.Combine(ModuleDirectory, "lib");

		if (Target.Platform == UnrealTargetPlatform.Win64)
		{
			string Dir = Path.Combine(Lib, "Win64");
			PublicAdditionalLibraries.Add(Path.Combine(Dir, "lingara_embed.dll.lib"));
			// Loaded explicitly at module startup, so a missing DLL disables
			// Lingara instead of failing the game (FLingaraEmbedModule).
			PublicDelayLoadDLLs.Add("lingara_embed.dll");
			RuntimeDependencies.Add("$(BinaryOutputDir)/lingara_embed.dll", Path.Combine(Dir, "lingara_embed.dll"));
		}
		else if (Target.Platform == UnrealTargetPlatform.Mac)
		{
			// Its install name is @rpath/liblingara_embed.dylib.
			string Dylib = Path.Combine(Lib, "Mac", "liblingara_embed.dylib");
			PublicAdditionalLibraries.Add(Dylib);
			RuntimeDependencies.Add("$(BinaryOutputDir)/liblingara_embed.dylib", Dylib);
		}
		else if (Target.Platform == UnrealTargetPlatform.Linux)
		{
			string So = Path.Combine(Lib, "Linux", "liblingara_embed.so");
			PublicAdditionalLibraries.Add(So);
			RuntimeDependencies.Add("$(BinaryOutputDir)/liblingara_embed.so", So);
		}
		else if (Target.Platform == UnrealTargetPlatform.Android)
		{
			// LingaraEmbed_UPL.xml copies it into libs/arm64-v8a/.
			PublicAdditionalLibraries.Add(Path.Combine(Lib, "Android", "arm64-v8a", "liblingara_embed.so"));
		}
		else if (Target.Platform == UnrealTargetPlatform.IOS)
		{
			// K3's As built: rustc's native-static-libs name no framework,
			// only the system libraries, of which the engine links all but iconv.
			PublicAdditionalLibraries.Add(Path.Combine(Lib, "IOS", "liblingara_embed.a"));
			PublicSystemLibraries.Add("iconv");
		}
	}
}
