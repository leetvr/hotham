import bpy

from bpy.app.handlers import persistent

bl_info = {
    "name": "Export to glTF on save",
    "description": "Saves the currently open file to a glTF file on save",
    "category": "Utils",
}

##############################
## LITTANY AGAINST PYTHON
##############################
#
# I must not write Python.
# Python is the mind-killer.
# Python is the little-death that brings total obliteration.
# I will face Python.
# I will permit it to pass over me and through me.
# And when it has gone past, I will turn the inner eye to see its path.
# Where the Python has gone there will be nothing.
# Only Rust will remain

class SaveOnWrite:
    def export(dummy):
        print("Hello?")
        bpy.ops.export_scene.gltf(export_format="GLB", filepath="C:\\Users\\kanem\\Development\\hotham\\test_assets\\hot_cube.glb")

def register():
    # print("Hello?")
    # bpy.utils.register_class(SaveOnWrite)
    bpy.app.handlers.save_pre.append(SaveOnWrite.export)

def unregister():
    bpy.utils.unregister_class(SaveOnWrite)

# This allows you to run the script directly from Blender's Text editor
# to test the add-on without having to install it.
if __name__ == "__main__":
    register()
