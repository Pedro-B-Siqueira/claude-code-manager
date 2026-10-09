import { describe, expect, it } from 'vitest';
import {
  describeUnrecognized,
  droppedPathsText,
  dropTargetContains,
  EnterGate,
  enterGateFor,
  escapeShellPath,
  isPlainEnter,
  pasteAction,
  shouldSubmitWithImages,
  splitDroppedPaths,
  type KeyLike,
} from './attachments';

function key(type: string, overrides: Partial<KeyLike> = {}): KeyLike {
  return { type, key: 'Enter', shiftKey: false, altKey: false, metaKey: false, ctrlKey: false, ...overrides };
}

const png = new File([new Uint8Array([0x89, 0x50])], 'image.png', { type: 'image/png' });
const heic = new File([new Uint8Array([0])], 'photo.heic', { type: 'image/heic' });

describe('pasteAction', () => {
  it('lets text through, sends a lone supported image to the tray and leaves the rest to Claude Code', () => {
    expect(pasteAction({ getData: () => 'hello', files: [png] })).toEqual({ kind: 'text' });
    expect(pasteAction({ getData: () => '', files: [png] })).toEqual({ kind: 'image', file: png });
    expect(pasteAction({ getData: () => '', files: [heic] })).toEqual({ kind: 'native' });
    expect(pasteAction({ getData: () => '', files: [] })).toEqual({ kind: 'native' });
    expect(pasteAction(null)).toEqual({ kind: 'text' });
  });
});

describe('dropped files', () => {
  it('separates images from other files by extension', () => {
    expect(splitDroppedPaths(['/a/shot.PNG', '/a/notes.md', '/a/photo.jpeg', '/a/clip.mov'])).toEqual({
      images: ['/a/shot.PNG', '/a/photo.jpeg'],
      others: ['/a/notes.md', '/a/clip.mov'],
    });
  });

  it('escapes paths the way Terminal does, keeping accented letters', () => {
    expect(escapeShellPath('/Users/dev/Área de Trabalho/relatório (1).pdf')).toBe('/Users/dev/Área\\ de\\ Trabalho/relatório\\ \\(1\\).pdf');
    expect(escapeShellPath('/tmp/plain-file_v2.ts')).toBe('/tmp/plain-file_v2.ts');
  });

  it('only accepts drops inside the terminal', () => {
    const rect = { left: 100, top: 50, right: 500, bottom: 400 };
    expect(dropTargetContains(rect, { x: 120, y: 60 })).toBe(true);
    expect(dropTargetContains(rect, { x: 99, y: 60 })).toBe(false);
  });
});

describe('Enter with images', () => {
  it('submits only on a plain Enter with images and no permission prompt', () => {
    expect(isPlainEnter(key('keydown'))).toBe(true);
    expect(isPlainEnter(key('keydown', { shiftKey: true }))).toBe(false);
    expect(isPlainEnter(key('keydown', { altKey: true }))).toBe(false);
    expect(isPlainEnter(key('keydown', { key: 'a' }))).toBe(false);
    expect(shouldSubmitWithImages(1, 'waiting')).toBe(true);
    expect(shouldSubmitWithImages(1, 'working')).toBe(true);
    expect(shouldSubmitWithImages(0, 'idle')).toBe(false);
    expect(shouldSubmitWithImages(2, 'permission')).toBe(false);
  });

  it('swallows the keypress and keyup of an intercepted Enter', () => {
    const gate = new EnterGate();
    expect(gate.decide(key('keydown'), true)).toBe('submit');
    expect(gate.decide(key('keypress'), false)).toBe('swallow');
    expect(gate.decide(key('keyup'), false)).toBe('swallow');
  });

  it('lets a normal Enter through, keypress included', () => {
    const gate = new EnterGate();
    expect(gate.decide(key('keydown'), false)).toBe('pass');
    expect(gate.decide(key('keypress'), false)).toBe('pass');
  });

  it('swallows any Enter while images are being sent', () => {
    const gate = new EnterGate();
    gate.decide(key('keydown'), true);
    gate.begin();
    expect(gate.decide(key('keydown'), false)).toBe('swallow');
    expect(gate.decide(key('keypress'), false)).toBe('swallow');
    gate.finish();
    expect(gate.decide(key('keydown'), false)).toBe('pass');
  });

  it('explains what Claude Code did not recognize', () => {
    expect(describeUnrecognized(0, 2)).toBe('O Claude Code não reconheceu as imagens. Confira o prompt e aperte Enter.');
    expect(describeUnrecognized(1, 3)).toBe('O Claude Code reconheceu 1 de 3 imagens. Confira o prompt e aperte Enter.');
  });
});

describe('per-session state', () => {
  it('keeps one Enter gate per session, surviving the terminal being re-created', () => {
    expect(enterGateFor('a')).toBe(enterGateFor('a'));
    expect(enterGateFor('a')).not.toBe(enterGateFor('b'));
  });

  it('types dropped files as escaped paths followed by a space, like Terminal', () => {
    expect(droppedPathsText(['/a/my notes.md', '/b/x.ts'])).toBe('/a/my\\ notes.md /b/x.ts ');
  });
});

