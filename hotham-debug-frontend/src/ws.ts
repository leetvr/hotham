import { Message } from './App';
import * as React from 'react';
import { db } from './db';

export const SERVER_ADDRESS = `ws://localhost:8000`;
export function createOnMessage(
  setSelectedSessionId: React.Dispatch<React.SetStateAction<string>>,
  setFramesReceived: React.Dispatch<React.SetStateAction<number>>
): (e: MessageEvent) => Promise<void> {
  return async (e: MessageEvent) => {
    const { init, frame }: Message = JSON.parse(e.data);
    if (frame) {
      try {
        await db.frames.add(frame);
        setFramesReceived((f) => f + 1);
      } catch (e) {
        console.error('Error adding frame', frame, e);
      }
      return;
    }

    if (init) {
      try {
        await db.sessions.add({ id: init.sessionId, timestamp: new Date() });
        setSelectedSessionId(init.sessionId);
      } catch (e) {
        console.error('Unable to add session', init.sessionId, e);
        return;
      }

      try {
        await db.frames.add(init.firstFrame);
        setFramesReceived((f) => f + 1);
      } catch (e) {
        console.error('Unable to add initial frame', init.firstFrame, e);
      }
      return;
    }
  };
}
