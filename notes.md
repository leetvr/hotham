# 20210621
fish eye lenses on quest 1 hmd (regardless of quest native or quest+link) after switching to using a reverse-z infinite far plane projection matrix. 

solution: I had made another tweak to my projection matrix calculation which I overlooked: I calculated the projection matrix with +y = up, and then inverted the result's [1][1] member to get +y = down. 

This isn't a problem on the index when top-fov == -down-fov. but on quest, that isn't true, and so it was using the wrong fov for the top and bottom planes. So I had to swap the top/bottom fov's when calculating before making the inverse-y switch.


# 20210624

06-25 17:32:54.816  5083  5107 I RustStdoutStderr: [View 0]: (Vector3 [0.039701987, 0.76085263, -0.774329], Quaternion { v: Vector3 [-0.010506645, -0.58130753, 0.009680491], s: -0.81355846 })
06-25 17:32:54.816  5083  5107 I RustStdoutStderr: [View 1]: (Vector3 [0.06206919, 0.7606085, -0.83964473], Quaternion { v: Vector3 [-0.010506645, -0.58130753, 0.009680491], s: -0.81355846 })