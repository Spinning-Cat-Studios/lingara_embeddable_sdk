// The Runtime module (ADR 1.10.26ac D3). On Win64 it loads K3's DLL
// explicitly before any ABI call, so a missing library leaves the subsystem
// Disabled instead of failing the game. Everywhere else the library is a
// launch-time dependency (dyld and ld.so have no delay-load), and an iOS
// build links the static library into the executable.
#pragma once

#include "CoreMinimal.h"
#include "Modules/ModuleInterface.h"

class LINGARAEMBED_API FLingaraEmbedModule : public IModuleInterface
{
public:
	virtual void StartupModule() override;
	virtual void ShutdownModule() override;

	// Loads lingara_embed.dll from Dir on Win64 and records the result; a
	// no-op that succeeds on every other platform. Never asserts.
	static bool TryLoadLibrary(const FString& Dir);

	static bool IsLibraryLoaded();
	// Why the load failed; empty when it did not.
	static const FString& LoadFailure();

	// Where RuntimeDependencies staged the DLL: the plugin's Binaries/Win64
	// in a modular editor build, the executable's directory in a monolithic one.
	static FString LibraryDirectory();
};
