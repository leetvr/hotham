# Introduction
This example is a clone of the popular VR game, [Beat Saber](https://beatsaber.com/). We're not lawyers, but this is provided as an example *only*: trying to upload this game to an app store is probably not a good idea.

# Running the example
## Pre-requisites
1. [Git LFS](https://git-lfs.github.com/) needs to be enabled for this repo. You'll have a _real_ bad time if you don't.
1. Install [Rust](https://www.rust-lang.org/tools/install), [cargo apk](https://crates.io/crates/cargo-apk) and the [Android SDK and Android NDK](https://developer.android.com/studio). _Note: only the Android NDK and SDK are required - if Android Studio is not your cup of tea, you can certainly install those separately_
1. An Oculus developer account is required & [developer mode](https://developer.oculus.com/documentation/native/android/mobile-device-setup/) needs to be enabled.

With all that done, you should now be able to see your device from ADB:

````
user@host:~/hotham$ adb devices
List of devices attached
1PASH9B12E0092	device
````

Then run `run_on_device.ps1` (Windows) or `run_on_device.sh` (Linux) and you're good to go.

# Troubleshooting
This is definitely _cutting edge_ software, so don't be surprised if it breaks. If you run into any trouble, your friends at the [Hotham discord](https://discord.gg/SZEZUX6ZsQ) can give you a hand!

# License Information
This example uses a modified version of the asset [Beat Sabers](https://sketchfab.com/3d-models/beat-sabers-e7c6358273d44faea03fa77d9792fd6a), by "Spark", under the [CC4.0 license](https://creativecommons.org/licenses/by/4.0/).
