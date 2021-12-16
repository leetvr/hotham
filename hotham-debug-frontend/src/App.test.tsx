import 'fake-indexeddb/auto';
import { act, render, waitFor, within } from '@testing-library/react';
import App, { Frame, Message } from './App';
import userEvent from '@testing-library/user-event';
import { db } from './db';
import WS from 'jest-websocket-mock';
import { SERVER_ADDRESS } from './ws';

function MockScrubber(props: {
  min: number;
  max: number;
  value: number;
  onScrubChange: (n: number) => void;
}): JSX.Element {
  const { max, onScrubChange } = props;
  return (
    <div
      data-testid="scrubber"
      onClick={() => {
        console.log('onScrubChange', max);
        onScrubChange(max);
      }}
    />
  );
}

jest.mock('./components/Viewer.tsx', () => ({
  Viewer: () => null,
}));

jest.mock('react-scrubber', () => ({
  Scrubber: MockScrubber,
}));

jest.mock('ws', () => {});

const stubFrames: Frame[] = [
  {
    sessionId: '0',
    id: 'abc123',
    frameNumber: 0,
    entities: {
      0: {
        id: 0,
        name: 'Test Entity 1',
        transform: {
          translation: [0, 0, 0],
          rotation: [0, 0, 0],
          scale: [1, 1, 1],
        },
        collider: {
          colliderType: 'cube',
          geometry: [1, 2, 3],
          translation: [0, 0.5, 0],
        },
      },
      1: {
        id: 1,
        name: 'Test Entity 2',
      },
    },
  },
  {
    sessionId: '0',
    id: 'abc456',
    frameNumber: 1,
    entities: {
      0: {
        id: 0,
        name: 'Test Entity 3',
      },
    },
  },
  {
    sessionId: '1',
    id: 'fafa123',
    frameNumber: 0,
    entities: {},
  },
];

async function clean() {
  WS.clean();
  await db.sessions.clear();
  await db.frames.clear();
}

async function setup() {
  await clean();
  await db.sessions.bulkAdd([
    { id: '0', timestamp: new Date() },
    { id: '1', timestamp: new Date() },
  ]);

  await db.frames.bulkAdd(stubFrames);
}

async function setupAndRender() {
  await setup();
  return render(<App />);
}

async function setupWithWebSocket() {
  await clean();
  return new WS(SERVER_ADDRESS, { jsonProtocol: true });
}

const DATE_REGEX = new RegExp(/\d{1,2}\/\d{1,2}\/\d{4}/);

test('renders a list of sessions when not connected', async () => {
  const { getByText } = await setupAndRender();
  const sessionContainer = getByText(/Previous sessions/i).parentElement;
  const sessionDate = await within(sessionContainer!).findAllByText(DATE_REGEX);
  expect(sessionDate).toHaveLength(2);
});

test('does not show sessions when connected', async () => {
  await setup();
  const server = new WS(SERVER_ADDRESS);
  const { getByText } = render(<App />);
  await server.connected;

  expect(getByText(/Connected to device/i)).toBeInTheDocument();
});

test('sends an INIT message when first connected', async () => {
  const server = await setupWithWebSocket();
  render(<App />);
  await server.connected;

  await expect(server).toReceiveMessage({ command: 1 });
});

test('the entity window gets populated with the first frame', async () => {
  const server = await setupWithWebSocket();
  const { getByText } = render(<App />);
  await server.connected;
  await server.nextMessage;
  const message: Message = {
    init: {
      sessionId: '5',
      firstFrame: {
        id: 'f0f0f0',
        frameNumber: 0,
        sessionId: '5',
        entities: {
          0: {
            id: 0,
            name: 'Test Entity 4',
          },
        },
      },
    },
  };

  act(() => {
    server.send(message);
  });

  const entitiesContainer = getByText(/Entities/i).parentElement;
  expect(
    await within(entitiesContainer!).findByText('Test Entity 4')
  ).toBeInTheDocument();
});

test('when multiple frames have been received, on the scrubber changes the frame', async () => {
  const server = await setupWithWebSocket();
  const { getByText, getByTestId } = render(<App />);
  await server.connected;
  await server.nextMessage;
  const message: Message = {
    init: {
      sessionId: '5',
      firstFrame: {
        id: 'f0f0f0',
        frameNumber: 0,
        sessionId: '5',
        entities: {
          0: {
            id: 0,
            name: 'Test Entity 4',
          },
        },
      },
    },
  };

  act(() => {
    server.send(message);
  });

  const entitiesContainer = getByText(/Entities/i).parentElement;
  expect(
    await within(entitiesContainer!).findByText('Test Entity 4')
  ).toBeInTheDocument();

  const message2: Message = {
    frame: {
      id: 'fafafa',
      frameNumber: 1,
      sessionId: '5',
      entities: {
        0: {
          id: 0,
          name: 'Test Entity 5',
        },
      },
    },
  };
  // const message2 = JSON.parse(
  //   `{"frame":{"id":"c9315b5a-bcf6-470d-aee0-7d2707ca41e5","frameNumber":0,"sessionId":"5","entities":{"160":{"name":"Red Saber","id":160,"transform":{"translation":[0.0,0.0,0.0],"rotation":[0.0,-0.0,0.0],"scale":[1.0,1.0,1.0]},"collider":null},"176":{"name":"Environment","id":176,"transform":{"translation":[0.0,12.998371,0.0],"rotation":[0.0,-0.0,0.0],"scale":[1.0,1.0,1.0]},"collider":null},"112":{"name":"Blue Cube","id":112,"transform":{"translation":[0.000007787097,-0.00039562775,-0.03639865],"rotation":[0.0006904579,0.0000069336797,2.088354e-7],"scale":[1.0,1.0,1.0]},"collider":{"colliderType":"cube","geometry":[1.0,1.0,1.0]}},"144":{"name":"Blue Saber","id":144,"transform":{"translation":[0.0,0.0,0.0],"rotation":[0.0,-0.0,0.0],"scale":[1.0,1.0,1.0]},"collider":null},"192":{"name":"Ramp","id":192,"transform":{"translation":[0.0,0.0,-32.697006],"rotation":[0.0,-0.0,0.0],"scale":[0.70659745,1.0,1.0]},"collider":null},"128":{"name":"Red Cube","id":128,"transform":{"translation":[0.0,0.0,0.0],"rotation":[0.0,-0.0,0.0],"scale":[1.0,1.0,1.0]},"collider":null}}}}`
  // );

  act(() => {
    server.send(message2);
  });

  const timeline = getByTestId('scrubber');

  await waitFor(async () =>
    expect(getByText('Frame 1 / 2')).toBeInTheDocument()
  );

  act(() => {
    userEvent.click(timeline);
  });

  expect(
    await within(entitiesContainer!).findByText('Test Entity 5')
  ).toBeInTheDocument();
});

test('clicking on a session changes the selected session', async () => {
  const { getByText } = await setupAndRender();
  const sessionContainer = getByText(/Previous sessions/i).parentElement;
  const session = (
    await within(sessionContainer!).findAllByText(DATE_REGEX)
  )[0];

  act(() => {
    userEvent.click(session);
  });

  // Ensure the entities have loaded.
  const entitiesContainer = getByText(/Entities/i).parentElement;
  expect(
    await within(entitiesContainer!).findByText('Test Entity 1')
  ).toBeInTheDocument();
  expect(
    await within(entitiesContainer!).findByText('Test Entity 2')
  ).toBeInTheDocument();
});

test('clicking on an entity shows details about that entity', async () => {
  const { getByText, getByRole } = await setupAndRender();
  const sessionContainer = getByText(/Previous sessions/i).parentElement;
  const session = (
    await within(sessionContainer!).findAllByText(DATE_REGEX)
  )[0];

  act(() => {
    userEvent.click(session);
  });

  // Ensure the entities are in the EntityList.
  const entitiesContainer = getByText(/Entities/i).parentElement;
  const entity = await within(entitiesContainer!).findByText('Test Entity 1');

  act(() => {
    userEvent.click(entity);
  });

  // Ensure the entity's properties are visible.
  const inspectorContainer = getByRole('heading', {
    name: 'Inspector',
  }).parentElement;

  await waitFor(() => {
    expect(inspectorContainer).toHaveTextContent('name: Test Entity 1');
    expect(inspectorContainer).toHaveTextContent('translation: x: 0 y: 0 z: 0');
    expect(inspectorContainer).toHaveTextContent('rotation: x: 0 y: 0 z: 0');
    expect(inspectorContainer).toHaveTextContent('scale: x: 1 y: 1 z: 1');
    expect(inspectorContainer).toHaveTextContent(
      'translation: x: 0 y: 0.5 z: 0'
    );
  });
});

test('clicking on the frame slider changes the current frame', async () => {
  const { getByText, getByTestId } = await setupAndRender();
  const sessionContainer = getByText(/Previous sessions/i).parentElement;
  const session = (
    await within(sessionContainer!).findAllByText(DATE_REGEX)
  )[0];

  act(() => {
    userEvent.click(session);
  });

  // Wait for the first session to load..
  const entitiesContainer = getByText(/Entities/i).parentElement;
  await within(entitiesContainer!).findByText('Test Entity 2');

  // Now, click on the scrubber so it loads the next frame..
  const timeline = getByTestId('scrubber');
  act(() => {
    userEvent.click(timeline);
  });

  const entity = await within(entitiesContainer!).findByText('Test Entity 3');
  expect(entity).toBeInTheDocument();
});
