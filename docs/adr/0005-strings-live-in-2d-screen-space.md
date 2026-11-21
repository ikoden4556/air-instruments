# Strings live in 2D screen space

Strings, Anchors and Pluckers are positioned in the flat, mirrored camera image, not in 3D. A single phone camera estimates hand depth poorly, so 3D Strings would make Plucks less accurate, and the reference video is 2D too. Distance to the camera therefore doesn't matter: a Plucker that crosses a String on screen Plucks it, even if the hand is in front of or behind where the String "is".
