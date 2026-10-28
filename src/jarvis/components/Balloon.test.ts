import { fireEvent, render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import type { JarvisSession } from '../data/jarvis-session';
import Balloon from './Balloon.svelte';

const SESSION: JarvisSession = {
  id: 's1',
  repo: 'atlas-web',
  name: 'remove-legacy-default-template',
  status: 'working',
  action: 'Bash · yarn ngc --noEmit -p control-panel',
  branch: 'feat-remove-legacy-template',
  lastActivityAt: 0,
};

function props(overrides: Partial<{ session: JarvisSession; selected: boolean; interactive: boolean; showAction: boolean; recentEvents: string[] | null }> = {}) {
  return { session: SESSION, elapsed: '4m 43s', selected: false, interactive: true, showAction: true, recentEvents: null, onToggle: vi.fn(), onOpen: vi.fn(), ...overrides };
}

describe('Balloon', () => {
  it('shows the session name, the elapsed time, the status, the repo and the current action', () => {
    render(Balloon, props());
    expect(screen.getByRole('button', { name: /remove-legacy-default-template/ })).toHaveAttribute('aria-expanded', 'false');
    expect(screen.getByText('4m 43s')).toBeInTheDocument();
    expect(screen.getByText('Trabalhando')).toBeInTheDocument();
    expect(screen.getByText('atlas-web')).toBeInTheDocument();
    expect(screen.getByText('Bash · yarn ngc --noEmit -p control-panel')).toBeInTheDocument();
  });

  it('toggles on a click on its header', async () => {
    const onToggle = vi.fn();
    render(Balloon, { ...props(), onToggle });
    await fireEvent.click(screen.getByRole('button', { name: /remove-legacy-default-template/ }));
    expect(onToggle).toHaveBeenCalledOnce();
  });

  it('expands into the branch, the last two events and a way back to the session', async () => {
    const onOpen = vi.fn();
    render(Balloon, { ...props({ selected: true, recentEvents: ['Leu timeline.service.ts', 'Rodou yarn lint'] }), onOpen });
    expect(screen.getByText('feat-remove-legacy-template')).toBeInTheDocument();
    expect(screen.getByText('Leu timeline.service.ts')).toBeInTheDocument();
    expect(screen.getByText('Rodou yarn lint')).toBeInTheDocument();
    await fireEvent.click(screen.getByRole('button', { name: 'Abrir sessão' }));
    expect(onOpen).toHaveBeenCalledOnce();
  });

  it('stays still and without the action while locked with details off', () => {
    render(Balloon, props({ interactive: false, showAction: false }));
    expect(screen.queryByRole('button')).toBeNull();
    expect(screen.getByText('remove-legacy-default-template')).toBeInTheDocument();
    expect(screen.queryByText('Bash · yarn ngc --noEmit -p control-panel')).toBeNull();
  });

  it('clips a long action', () => {
    render(Balloon, props({ session: { ...SESSION, action: `Bash · ${'x'.repeat(150)}` } }));
    expect(screen.getByText(/^Bash · x+…$/).textContent).toHaveLength(96);
  });
});
