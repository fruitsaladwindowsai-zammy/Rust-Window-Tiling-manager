# Overview Mode (`Win+O`)

The **Win+O** overview provides a live, interactive bird's-eye view of the active monitor and workspace.

![Overview Demo](../assets/overview-demo.gif)

## Visual Design & Animations

| Live Acrylic Overview | Card Navigation & Selection |
|:---:|:---:|
| ![Overview Mode](../assets/overview-mode.jpg) | ![Overview Navigation](../assets/overview-navigation.jpg) |

- **Dynamic Fit-to-Display Layout**: Window thumbnails scale automatically based on the number of open windows so the entire layout fits cleanly within the display view without scrolling.
- **Frameless App Previews + Bottom-Center Icons**: Renders live DWM thumbnails directly over Gaussian-blurred desktop wallpaper (`pure_blur` or `acrylic_clear`) with no opaque card box or titlebar text — displaying each app's icon centered along the bottom edge (plus stack dots when a cell holds multiple windows).
- **Front/Back Transitions**: Pressing **Win+O** animates the active window backward into the grid; clicking a card or pressing `Enter` animates the highlighted window smoothly back to the front.

## Highlighted Close Button

- Only the **currently highlighted** card activates the close button (`hit_close_button` + `close_springs`).
- Rendered as a vivid **red circle with a white `✕`** in the **top-right corner** of the card, clamped inside screen bounds (`close_button_rect`).
- Animates in and out with dedicated spring physics and scales slightly when hovered.
- Automatically hides while dragging a card (`active_drag`).

## Drag-and-Drop Reordering & Snapping

![Tiling and Snapping Demo](../assets/tiling-snap-demo.gif)

- Dragging a window card shrinks the overview tiles smoothly and reveals available target grid slots.
- Drop onto any occupied or empty coordinate to move or swap the window.
- **Drag-to-Snap Horizontal Split**: Drop any window card onto another window to snap both together side-by-side ($50\% / 50\%$ width) into a unified view with infinite scroll between views!
- Hold **`Shift`** while dropping to stack the window into the target cell.

| Snapped Dual-App Horizontal Split View |
|:---:|
| ![Split Horizontal Snap](../assets/split-horizontal-snap.png) |
