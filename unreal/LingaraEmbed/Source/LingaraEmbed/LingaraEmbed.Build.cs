// The Runtime module: the subsystem, the decoder and encoder, the settings,
// the two async actions and the automation specs (ADR 1.10.26ac D2, D5–D7).
using System.IO;
using UnrealBuildTool;

public class LingaraEmbed : ModuleRules
{
	public LingaraEmbed(ReadOnlyTargetRules Target) : base(Target)
	{
		PCHUsage = PCHUsageMode.UseExplicitOrSharedPCHs;
		PublicDependencyModuleNames.AddRange(new string[] { "Core", "CoreUObject", "Engine", "DeveloperSettings", "LingaraEmbedLibrary" });
		PrivateDependencyModuleNames.AddRange(new string[] { "Json", "HTTP", "Projects" });

		if (Target.Platform == UnrealTargetPlatform.Android)
		{
			AdditionalPropertiesForReceipt.Add("AndroidPlugin", Path.Combine(ModuleDirectory, "LingaraEmbed_UPL.xml"));
		}
	}
}
