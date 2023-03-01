# Hotham OpenXR Client
This package will eventually replace the `hotham-simulator` package, along with `hotham-editor`.

Currently, `hotham-simulator` does a lot. It:

- Implements (most) of the OpenXR runtime
- It acts as an OpenXR server
- It controls the window and inputs for the OpenXR runtime

This is a bit too much and is mostly a result of Kane's misunderstanding of how OpenXR works.

# The new model
In The New Model, which is shiny and perfect and has no flaws, the OpenXR client is much more limited in scope.

- It creates a dynamic library that the OpenXR loader can call into
- It handles Vulkan instance and device creation on behalf of the OpenXR app
- It communicates with the OpenXR server (that is, `hotham-editor`) to submit frames and check for input events

You can see a flowchart of how this all works [here](https://www.figma.com/file/5kDF7s5wNewPQY7hw1WXsd/Hotham-Editor?node-id=0%3A1&t=iZ09gupiiA5nqFYR-1)
