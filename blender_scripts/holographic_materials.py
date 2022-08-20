# This example assumes we have a mesh object selected

import bpy
import mathutils
import math
import bmesh
import numpy as np

# Get the active mesh
me = bpy.context.object.data

# Get a BMesh from this mesh, the mesh must already be in editmode.
bm = bmesh.from_edit_mesh(me)

# Fit quadric to mesh vertices
# f(x,y,z) = pᵀ·Q·p,  pᵀ = [x, y, x, 1]
# f(x,y,z) = 0
# pᵀ·Q·p = 0
# Aq = 0,  dim(A) = [m, 10],  dim(q) = [10, 1]
# A.row(i) = [x·x, y·y, z·z, x·y, x·z, y·z, x, y, z, 1]
# qᵀ = [q_11, q_22, q_33, q_12+q_21, q_13+q_31, q_23+q_32, q_14+q_41, q_24+q_42, q_34+q_43, q_44]
# AᵀA·q = 0
# q ∈ ker(AᵀA)
# q = v_1·c_1 + v_2·c_2 + ...  where A·v_1 = 0, A·v_2 = 0 ...
# q = V·c,  dim(V) = [10, k],  dim(c) = [k,1]

# ∇f(x,y,z) ~= n
# df(p)/dx = 2·x·q_11 + y·(q_12+q_21) + z·(q_13+q_31) + q_14+q_41
# df(p)/dy = 2·y·q_22 + x·(q_12+q_21) + z·(q_23+q_32) + q_24+q_42
# df(p)/dz = 2·z·q_33 + x·(q_13+q_31) + y·(q_23+q_32) + q_34+q_43
# Bq = N,  dim(B) = [3m, 10],  dim(N) = [3m, 1]
# B.row(i) = [2·x, 0, 0, y, z, 0, 1, 0, 0, 0]
# B.row(i) = [0, 2·y, 0, x, 0, z, 0, 1, 0, 0]
# B.row(i) = [0, 0, 2·z, 0, x, y, 0, 0, 1, 0]

# B·V·c = N
# (B·V)ᵀ·B·V·c = (B·V)ᵀ·N
# Vᵀ·Bᵀ·B·V·c = Vᵀ·Bᵀ·N
# Vᵀ·(Bᵀ·B)·V·c = Vᵀ·(Bᵀ·N)
# c = (Vᵀ·(Bᵀ·B)·V)⁻¹ · Vᵀ·(Bᵀ·N)

# ∇f(p) = (Qᵀ+Q)·p
# Q is symmetric => ∇f(p) = 2·Q·p

AtA = np.zeros((10, 10))
BtB = np.zeros((10, 10))
BtN = np.zeros((10, 1))
for v in bm.verts:
    if v.select:
        x, y, z = (v.co.x, v.co.y, v.co.z)
        a_row = np.array(
            [
                [x * x, y * y, z * z, x * y, x * z, y * z, x, y, z, 1],
            ]
        )
        AtA += a_row.T * a_row
        b_rows = np.array(
            [
                [2 * x, 0, 0, y, z, 0, 1, 0, 0, 0],
                [0, 2 * y, 0, x, 0, z, 0, 1, 0, 0],
                [0, 0, 2 * z, 0, x, y, 0, 0, 1, 0],
            ]
        )
        BtB += b_rows.T @ b_rows
        BtN += b_rows.T @ np.array([[v.normal.x], [v.normal.y], [v.normal.z]])

# The eigenvectors are arranged as columns
eigenvalues, eigenvectors = np.linalg.eigh(AtA)

# Measure size of nullspace
k = np.count_nonzero(eigenvalues < 1.0e-10)
if k == 0:
    print("WARNING: This shape does not look like a quadric, the fit will be bad!")
    k = 1
V = eigenvectors[:, :k]

# Pick solution within the nullspace based on the vertex normals
u, s, vh = np.linalg.svd(V.T @ BtB @ V, hermitian=True)
if np.any(s < 1.0e-10):
    print(
        "INFO: Normals do not constrain the solution fully, adding prior for regularization."
    )
    u, s, vh = np.linalg.svd(V.T @ (BtB + np.identity(10) * 1.0e-5) @ V, hermitian=True)
c = vh.T @ np.diag(1 / s) @ u.T @ V.T @ BtN
q = V @ c

print("eigenvalues: ", eigenvalues)
print("k: ", np.count_nonzero(eigenvalues < 1.0e-10))
print("V: ", V)
print("c: ", c)
print("q: ", q)


# bm.free()  # free and prevent further access
