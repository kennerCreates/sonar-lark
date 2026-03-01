# Camera System

## Camera Switching

`CameraState` holds `CameraMode` (Chase/Fpv/Spectator/CourseCamera(usize)). Keys: 1-9,0=CourseCamera(0..8) if present (else 1=fallback Chase), 2=Chase always, Shift+F=FPV (cycles target on repeat), Shift+S=Spectator. `CourseCameras` resource built from `CourseData.cameras` at race start (primary at index 0). Default mode: `CourseCamera(0)` if cameras exist, else Chase. Each mode has its own update system gated by `camera_mode_is()` / `camera_mode_is_course_camera()`. `CameraHudRoot` shows mode/hints with dynamic camera labels.

## Chase Camera

Broadcast-style pack-follow camera. Chase follows leader with pack blending. `ChaseState` resource holds smoothed center/velocity.

## FPV Camera

Stabilized close-follow camera on target drone. Cycles target on repeat Shift+F press. `FpvFollowState` resource.

## Spectator Camera

RTS-style orbit controls: middle-mouse orbit, scroll zoom, WASD pan. `SpectatorOrbitState` resource. `SpectatorSettings` for movement speed + mouse sensitivity.

## Course Cameras

CourseCamera snaps to stored transform. `CourseCameraEntry` holds pre-computed Transform + optional label.
