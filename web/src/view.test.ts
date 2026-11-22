import { expect, it } from 'vitest';
import { coverMapping } from './view';

it('a 4:3 camera image covering a 2:1 canvas is cropped top and bottom, centred', () => {
  const map = coverMapping(640, 480, 2000, 1000);
  // Scaled to 2000 wide: 1500 tall, 250 cropped off each of top and bottom.
  expect(map({ x: 0.5, y: 0.5 })).toEqual({ x: 1000, y: 500 });
  expect(map({ x: 0, y: 0 })).toEqual({ x: 0, y: -250 });
  expect(map({ x: 1, y: 1 })).toEqual({ x: 2000, y: 1250 });
});
