{ pkgs ? import <nixpkgs> {} }:

# based on https://nixos.org/manual/nixpkgs/stable/#android
# note: if you get any weird errors of "required extensions not found" or the like, somehow hotham
# doesn't search /run/opengl-driver
# you can set VK_ICD_FILENAMES manually instead, see https://nixos.org/manual/nixos/stable/index.html#sec-gpu-accel-vulkan
let
  cmakeVersion = "3.22.1";

  androidComposition = pkgs.androidenv.composeAndroidPackages {
    includeNDK = true;
    ndkVersion = "22.1.7171670";
    platformVersions = ["28"];
    cmakeVersions = [cmakeVersion];
  };

  openXrDropin = {
    file_format_version = "1.0.0";
    runtime = {
      api_version = "1.0";
      name = "Hotham Simulator";
      # this won't work with pure flakes but flakes are unstable anyway
      library_path = "${toString ./.}/target/debug/libhotham_simulator.so";
    };
  };
in
pkgs.mkShell rec {
  buildInputs = with pkgs; [
    rustup ninja cmake
    openssl pkg-config

    cargo-apk adoptopenjdk-bin

    shaderc python3
    vulkan-headers vulkan-loader
    vulkan-tools vulkan-tools-lunarg
    vulkan-validation-layers vulkan-extension-layer
    monado openxr-loader openxr-loader.dev

    libxkbcommon
    wayland xorg.libX11 xorg.libXcursor xorg.libXrandr xorg.libXi
    fontconfig freetype
    alsa-lib

    renderdoc
  ];

  # the nixpkgs manual lists that ANDROID_HOME is outdated and ANDROID_SDK_ROOT should be used instead
  # when actually running the android tools, they say the opposite, ANDROID_SDK_ROOT being outdated
  # so I just set both
  ANDROID_SDK_ROOT = "${androidComposition.androidsdk}/libexec/android-sdk";
  ANDROID_HOME = ANDROID_SDK_ROOT;
  ANDROID_NDK_ROOT = "${ANDROID_SDK_ROOT}/ndk-bundle";

  shellHook = ''
    export PATH="$(echo "$ANDROID_SDK_ROOT/cmake/${cmakeVersion}".*/bin):$PATH"
    export LD_LIBRARY_PATH="$LD_LIBRARY_PATH:${builtins.toString (pkgs.lib.makeLibraryPath buildInputs)}";
    export XR_RUNTIME_JSON="${builtins.toFile "hotham-openxr-runtime.json" (builtins.toJSON openXrDropin)}"

    rustup default 1.67 # required due to libunwind having fun
    rustup target add aarch64-linux-android
  '';
}
