import { Euler, Quaternion } from 'three';
import { Vector4 } from './App';

export function vec4toQuaternion([x, y, z, w]: Vector4): Quaternion {
  return new Quaternion(x, y, z, w);
}

export function vec4toEuler([x, y, z, w]: Vector4): Euler {
  const q = new Quaternion(x, y, z, w);
  const e = new Euler();
  e.setFromQuaternion(q);
  return e;
}
