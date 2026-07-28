import type { Modifier } from "@dnd-kit/core";
import { getEventCoordinates } from "@dnd-kit/utilities";

/**
 * Keeps the center of the drag overlay under the pointer.
 *
 * Needed when the overlay is smaller than the source card (compact preview):
 * dnd-kit's default grab-point offset leaves the cursor beside the shrunk item.
 */
export const snapCenterToCursor: Modifier = ({
  activatorEvent,
  activeNodeRect,
  draggingNodeRect,
  transform,
}) => {
  const nodeRect = draggingNodeRect ?? activeNodeRect;
  if (!nodeRect || !activatorEvent) {
    return transform;
  }

  const activatorCoordinates = getEventCoordinates(activatorEvent);
  if (!activatorCoordinates) {
    return transform;
  }

  const offsetX = activatorCoordinates.x - nodeRect.left;
  const offsetY = activatorCoordinates.y - nodeRect.top;

  return {
    ...transform,
    x: transform.x + offsetX - nodeRect.width / 2,
    y: transform.y + offsetY - nodeRect.height / 2,
  };
};
