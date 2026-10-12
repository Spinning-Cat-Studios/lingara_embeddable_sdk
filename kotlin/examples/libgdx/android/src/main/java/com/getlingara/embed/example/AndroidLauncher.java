package com.getlingara.embed.example;

import android.os.Bundle;
import com.badlogic.gdx.backends.android.AndroidApplication;
import com.badlogic.gdx.backends.android.AndroidApplicationConfiguration;

/**
 * The Android launcher (ADR 1.10.26ae D8): lingara-embed-natives-android supplies the arm64
 * library. Java, so this module needs no Kotlin compiler (see gradle.properties). libGDX's
 * pause() and resume() reach LingaraDemo, which pauses and resumes the stream.
 */
public class AndroidLauncher extends AndroidApplication {
  @Override
  protected void onCreate(Bundle savedInstanceState) {
    super.onCreate(savedInstanceState);
    initialize(new LingaraDemo(), new AndroidApplicationConfiguration());
  }
}
