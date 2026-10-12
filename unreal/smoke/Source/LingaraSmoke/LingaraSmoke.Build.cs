// The smoke project's one module. It depends on LingaraEmbed and HTTP so the
// snippet's two files, copied in by smoke.sh, compile (ADR 1.10.26ac D8).
using UnrealBuildTool;

public class LingaraSmoke : ModuleRules
{
	public LingaraSmoke(ReadOnlyTargetRules Target) : base(Target)
	{
		PCHUsage = PCHUsageMode.UseExplicitOrSharedPCHs;
		PublicDependencyModuleNames.AddRange(new string[] { "Core", "CoreUObject", "Engine", "LingaraEmbed", "HTTP", "Json" });
	}
}
