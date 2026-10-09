import { describe, expect, it } from 'vitest';
import { cardClickFocuses, isInteractiveTarget } from './card-click';

describe('cardClickFocuses', () => {
  it('opens the session for a plain click on the card', () => {
    expect(cardClickFocuses({ fromInteractive: false, pointerTravel: 0, hasSelection: false })).toBe(true);
    expect(cardClickFocuses({ fromInteractive: false, pointerTravel: 4, hasSelection: false })).toBe(true);
  });

  it('leaves buttons, drags and text selection alone', () => {
    expect(cardClickFocuses({ fromInteractive: true, pointerTravel: 0, hasSelection: false })).toBe(false);
    expect(cardClickFocuses({ fromInteractive: false, pointerTravel: 12, hasSelection: false })).toBe(false);
    expect(cardClickFocuses({ fromInteractive: false, pointerTravel: 0, hasSelection: true })).toBe(false);
  });
});

describe('isInteractiveTarget', () => {
  it('recognizes controls and anything inside them', () => {
    document.body.innerHTML = '<article><p id="text">x</p><button><svg id="icon"></svg></button><div role="menu"><span role="menuitem" id="item">a</span></div><a href="#" id="link">l</a></article>';
    expect(isInteractiveTarget(document.getElementById('text'))).toBe(false);
    expect(isInteractiveTarget(document.getElementById('icon'))).toBe(true);
    expect(isInteractiveTarget(document.getElementById('item'))).toBe(true);
    expect(isInteractiveTarget(document.getElementById('link'))).toBe(true);
    expect(isInteractiveTarget(null)).toBe(false);
  });
});
