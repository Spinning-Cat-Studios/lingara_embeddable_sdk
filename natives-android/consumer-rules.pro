# The rules the Lingara Android app proves in production with JNA 5.19.1 and R8
# (ADR 1.10.26ae D5). Every Android game depends on this artefact, so they
# reach every minified build. The generated com.getlingara.embed package
# needs no rule of its own.
-keep class com.sun.jna.** { *; }
# Structure subclasses must keep their fields by name: JNA reads them
# reflectively in @Structure.FieldOrder's order.
-keep class * extends com.sun.jna.Structure { <fields>; <init>(...); }
-keep class * implements com.sun.jna.Callback { *; }
-dontwarn java.awt.**
