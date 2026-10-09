/** Past this, the pointer was dragging the card to reorder it, not clicking it. */
const DRAG_THRESHOLD_PX = 5;

const INTERACTIVE = 'button, a, input, select, textarea, [role="menu"], [role="menuitem"]';

export interface CardClick {
  fromInteractive: boolean;
  pointerTravel: number;
  hasSelection: boolean;
}

export function cardClickFocuses(click: CardClick): boolean {
  return !click.fromInteractive && click.pointerTravel <= DRAG_THRESHOLD_PX && !click.hasSelection;
}

export function isInteractiveTarget(target: EventTarget | null): boolean {
  return target instanceof Element && target.closest(INTERACTIVE) !== null;
}
