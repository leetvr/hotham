# Inverse Kinematics
Solved constraint by constraint iteratively.
The sum of all constraints forms a cost function.
The constraints have different weights with different order of magnitudes to approximate finding solutions for weaker constraints in the nullspace of stronger constraints.
The constraints are ranked from the strongest to the weakest:
1. Skeletal constraints (SphericalConstraint and AngularCardanConstraint)
2. Flexibility limit constraints (shoulder reach, knee angles, elbow angles, wrists, ankles)
3. Controller inputs (AnchorConstraint)
4. Balance (keep center of mass over the weight bearing feet)
5. Neutral joint angles (shoulders, half bent knees, elbows, wrists)
6. Pose from physics simulation (weakly incorporate collisions and gravity)
