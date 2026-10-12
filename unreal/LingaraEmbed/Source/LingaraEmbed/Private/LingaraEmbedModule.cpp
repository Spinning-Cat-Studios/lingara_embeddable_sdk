// Module startup and the Win64 library load (ADR 1.10.26ac D3). Shutdown
// never calls FreeDllHandle: K3's `free` detaches its I/O thread past 1 s,
// and unmapping the library then would pull code from under it. A library
// left loaded at process exit is the engine's own common practice.
#include "LingaraEmbedModule.h"

#include "HAL/PlatformProcess.h"
#include "Interfaces/IPluginManager.h"
#include "LingaraEmbedLog.h"
#include "Misc/Paths.h"
#include "Modules/ModuleManager.h"

DEFINE_LOG_CATEGORY(LogLingaraEmbed);

namespace
{
	bool GLoaded = !PLATFORM_WINDOWS;
	FString GFailure;
}

void FLingaraEmbedModule::StartupModule()
{
	TryLoadLibrary(LibraryDirectory());
}

void FLingaraEmbedModule::ShutdownModule()
{
}

FString FLingaraEmbedModule::LibraryDirectory()
{
#if IS_MONOLITHIC
	return FPlatformProcess::BaseDir();
#else
	const TSharedPtr<IPlugin> Plugin = IPluginManager::Get().FindPlugin(TEXT("LingaraEmbed"));
	return Plugin.IsValid() ? FPaths::Combine(Plugin->GetBaseDir(), TEXT("Binaries/Win64")) : FString();
#endif
}

bool FLingaraEmbedModule::TryLoadLibrary(const FString& Dir)
{
#if PLATFORM_WINDOWS
	const FString Path = FPaths::Combine(Dir, TEXT("lingara_embed.dll"));
	void* Handle = Dir.IsEmpty() ? nullptr : FPlatformProcess::GetDllHandle(*Path);
	GLoaded = Handle != nullptr;
	GFailure = GLoaded ? FString() : FString::Printf(TEXT("lingara_embed.dll could not be loaded from '%s'"), *Dir);
	if (!GLoaded)
	{
		UE_LOG(LogLingaraEmbed, Error, TEXT("%s: Lingara is disabled for this run"), *GFailure);
	}
	return GLoaded;
#else
	GLoaded = true;
	GFailure.Reset();
	return true;
#endif
}

bool FLingaraEmbedModule::IsLibraryLoaded()
{
	return GLoaded;
}

const FString& FLingaraEmbedModule::LoadFailure()
{
	return GFailure;
}

IMPLEMENT_MODULE(FLingaraEmbedModule, LingaraEmbed)
