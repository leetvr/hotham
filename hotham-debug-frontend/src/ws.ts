import { Message } from './App';
import * as React from 'react';
import { db } from './db';

export const SERVER_ADDRESS = `ws://localhost:8000`;
export function createOnMessage(
  setSelectedSessionId: React.Dispatch<React.SetStateAction<string>>,
  setFramesReceived: React.Dispatch<React.SetStateAction<number>>
): (e: MessageEvent) => Promise<void> {
  return async (e: MessageEvent) => {
    const { init, data }: Message = JSON.parse(e.data);
    if (data) {
      await db.frames.add(data);
      console.log('About to add frame', data);
      setFramesReceived((f) => f + 1);
    }

    if (init) {
      await db.sessions.add({ id: init.sessionId, timestamp: new Date() });
      console.log('About to add frame', init.data);
      await db.frames.add(init.data);
      setSelectedSessionId(init.sessionId);
      setFramesReceived((f) => f + 1);
    }
  };
}
