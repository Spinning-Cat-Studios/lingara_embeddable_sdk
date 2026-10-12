// The smoke project's game target (ADR 1.10.26ac D8).
using UnrealBuildTool;

public class LingaraSmokeTarget : TargetRules
{
	public LingaraSmokeTarget(TargetInfo Target) : base(Target)
	{
		Type = TargetType.Game;
		DefaultBuildSettings = BuildSettingsVersion.Latest;
		IncludeOrderVersion = EngineIncludeOrderVersion.Latest;
		ExtraModuleNames.Add("LingaraSmoke");
	}
}
