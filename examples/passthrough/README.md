# Passthrough example

This examples shows how to use [Passthrough](https://support.oculus.com/articles/in-vr-experiences/oculus-features/what-is-passthrough/) as an underlay, showing the real world beneath the virtual one.

Passthrough requires the following addition to `Cargo.toml`:
```toml
[[package.metadata.android.uses_feature]]
name = "com.oculus.feature.PASSTHROUGH"
required = true
version = 1
```

[Here](https://developer.oculus.com/documentation/native/android/mobile-passthrough/) are more details about how Passthrough works.