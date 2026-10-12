// The smoke project's editor target: the automation specs run in it (ADR 1.10.26ac D10).
using UnrealBuildTool;

public class LingaraSmokeEditorTarget : TargetRules
{
	public LingaraSmokeEditorTarget(TargetInfo Target) : base(Target)
	{
		Type = TargetType.Editor;
		DefaultBuildSettings = BuildSettingsVersion.Latest;
		IncludeOrderVersion = EngineIncludeOrderVersion.Latest;
		ExtraModuleNames.Add("LingaraSmoke");
	}
}
