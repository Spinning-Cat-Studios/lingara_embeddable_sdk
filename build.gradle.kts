// The root project of the shared Gradle build (ADR 1.10.26af D13, after the
// app kits' 30.9.26am D8). It builds nothing itself. Loading the convention
// plugins here, unapplied, puts them on the root classloader once, so a build
// service they register (Spotless's) is one service for every project rather
// than one per sibling. K7's projects apply the same plugins.
//
// The Android library plugin is loaded here too (ADR 1.10.26ag D6a): the
// publishing convention's plugin reads AGP's classes to publish
// natives-android's AAR, and it can see them only from the classloader it
// was loaded in. Loading needs no Android SDK; applying it does.

plugins {
    id("lingara.java-conventions") apply false
    id("lingara.publishing-conventions") apply false
    id("lingara.kotlin-conventions") apply false
    alias(libs.plugins.android.library) apply false
}
