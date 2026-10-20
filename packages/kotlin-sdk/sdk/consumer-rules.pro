# Keep JNI-facing classes: the native library resolves these by name from
# Rust (FindClass / GetMethodID), so R8 must not rename or strip them.
-keep class org.dashfoundation.dashsdk.ffi.** { *; }
