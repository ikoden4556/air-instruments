// On-screen Undo, Clear and Key: big targets along the top edge, out of the playing area.

export const KEY_NAMES = ['C', 'C♯', 'D', 'E♭', 'E', 'F', 'F♯', 'G', 'A♭', 'A', 'B♭', 'B'];

/** What the controls change in the Engine. */
export interface Controlled {
  undo(): void;
  clear(): void;
  setKey(keyRoot: number): void;
}

export type Control = { type: 'undo' } | { type: 'clear' } | { type: 'key'; root: number };

export function apply(control: Control, engine: Controlled): void {
  switch (control.type) {
    case 'undo':
      return engine.undo();
    case 'clear':
      return engine.clear();
    case 'key':
      return engine.setKey(((control.root % 12) + 12) % 12);
  }
}

/** Adds the controls to the page, wired to `engine`. */
export function mountControls(engine: Controlled, initialRoot = 0): HTMLElement {
  const bar = document.createElement('div');
  bar.id = 'controls';
  const button = (label: string, control: Control) => {
    const b = document.createElement('button');
    b.textContent = label;
    b.onclick = () => apply(control, engine);
    bar.append(b);
  };
  button('Undo', { type: 'undo' });
  button('Clear', { type: 'clear' });
  const key = document.createElement('select');
  key.setAttribute('aria-label', 'Key');
  KEY_NAMES.forEach((name, root) => key.add(new Option(`Key ${name}`, String(root))));
  key.value = String(initialRoot);
  key.onchange = () => apply({ type: 'key', root: Number(key.value) }, engine);
  bar.append(key);
  document.body.append(bar);
  return bar;
}
