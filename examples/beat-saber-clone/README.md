# Introduction
This example is a clone of the popular VR game, [Beat Saber](https://beatsaber.com/). We're not lawyers, but this is provided as an example *only*: trying to upload this game to an app store is probably not a good idea.

# Oculus Quest Starting Guide

Make sure [Git LFS](https://git-lfs.github.com/) is enabled on your local copy of this repository.

You will need to have [Rust](https://www.rust-lang.org/tools/install) and [cargo apk](https://crates.io/crates/cargo-apk) installed as well as the [Android SDK](https://developer.android.com/studio) to be able to compile the Android application.

An Oculus developer account is required & [developer mode](https://developer.oculus.com/documentation/native/android/mobile-device-setup/) must be enabled.

You should be able to see your device from ADB:

````
user@host:~/hotham$ adb devices
List of devices attached
1PASH9B12E0092	device
````

Then, running `run_on_device.ps1` (Windows) or `run_on_device.sh` (Linux) should compile the application, install it on the headset and start it.

# License Information
This example uses a modified version of the asset [Beat Sabers](https://sketchfab.com/3d-models/beat-sabers-e7c6358273d44faea03fa77d9792fd6a), by "Spark", under the [CC4.0 license](https://creativecommons.org/licenses/by/4.0/).