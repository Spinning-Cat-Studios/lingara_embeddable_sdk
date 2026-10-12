// The settings' constructor and category (ADR 1.10.26ac D7).
#include "LingaraEmbedSettings.h"

ULingaraEmbedSettings::ULingaraEmbedSettings()
{
	CategoryName = TEXT("Plugins");
	SectionName = TEXT("Lingara Embed");
}

FLingaraEmbedConfig ULingaraEmbedSettings::ToConfig() const
{
	FLingaraEmbedConfig Config;
	Config.BaseUrl = BaseUrl;
	Config.LogLevel = LogLevel;
	Config.MaxEventsPerTick = MaxEventsPerTick;
	return Config;
}
