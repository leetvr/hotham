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
    entities: [
      {
        id: '0',
        entityId: 0,
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
      {
        id: '1',
        entityId: 1,
        name: 'Test Entity 2',
      },
    ],
  },
  {
    sessionId: '0',
    id: 'abc456',
    frameNumber: 1,
    entities: [
      {
        id: '0',
        entityId: 0,
        name: 'Test Entity 3',
      },
    ],
  },
  {
    sessionId: '1',
    id: 'fafa123',
    frameNumber: 0,
    entities: [],
  },
];

const stubMessages: Message[] = [
  {
    init: {
      sessionId: '5',
      firstFrame: {
        id: 'f0f0f0',
        frameNumber: 0,
        sessionId: '5',
        entities: [
          {
            id: '0',
            entityId: 0,
            name: 'Test Entity 4',
          },
        ],
      },
    },
  },
  {
    frames: [
      {
        id: 'fafafa',
        frameNumber: 1,
        sessionId: '5',
        entities: [
          {
            id: '0',
            entityId: 0,
            name: 'Test Entity 5',
          },
        ],
      },
    ],
  },
];

async function clean() {
  WS.clean();
  await db.sessions.clear();
  await db.frames.clear();
}

afterEach(async () => {
  await clean();
});

async function setup() {
  await db.sessions.bulkAdd([
    { id: '0', timestamp: new Date('2021-01-01T01:00:00.000Z') },
    { id: '1', timestamp: new Date('2020-01-01T02:00:00.000Z') },
    { id: '2', timestamp: new Date('2021-01-01T00:00:00.000Z') },
  ]);

  await db.frames.bulkAdd(stubFrames);
}

async function setupAndRender() {
  await setup();
  return render(<App />);
}

async function setupWithWebSocket() {
  return new WS(SERVER_ADDRESS, { jsonProtocol: true });
}

async function setupWithMessagesFromServer(messages: Message[]) {
  const server = await setupWithWebSocket();
  const renderResult = render(<App />);
  await act(async () => {
    await server.connected;
  });
  await server.nextMessage;

  for (let message of messages) {
    act(() => {
      server.send(message);
    });
  }

  return {
    ...renderResult,
    server,
  };
}

const DATE_REGEX = new RegExp(/\d{1,2}\/\d{1,2}\/\d{4}/);

test('renders a list of sessions ordered in reverse chronological order when not connected', async () => {
  const { getByText } = await setupAndRender();
  const sessionContainer = getByText(/Previous sessions/i).parentElement;
  const sessionDate = await within(sessionContainer!).findAllByText(DATE_REGEX);
  expect(sessionDate[0]).toHaveTextContent('01/01/2021, 11:00:00 am');
  expect(sessionDate[1]).toHaveTextContent('01/01/2021, 10:00:00 am');
  expect(sessionDate[2]).toHaveTextContent('01/01/2020, 12:00:00 pm');
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

test('when connected, the entity window gets populated with the first frame', async () => {
  const { getByText } = await setupWithMessagesFromServer([stubMessages[0]]);
  const entitiesContainer = getByText(/Entities/i).parentElement;
  expect(
    await within(entitiesContainer!).findByText('Test Entity 4 (0)')
  ).toBeInTheDocument();
});

test('when multiple frames have been received, clicking on the scrubber changes the frame', async () => {
  const { getByText, getByTestId } = await setupWithMessagesFromServer(
    stubMessages
  );
  const entitiesContainer = getByText(/Entities/i).parentElement;

  expect(
    await within(entitiesContainer!).findByText('Test Entity 4 (0)')
  ).toBeInTheDocument();

  const timeline = getByTestId('scrubber');

  await waitFor(async () =>
    expect(getByText('Frame 1 / 2')).toBeInTheDocument()
  );

  act(() => {
    userEvent.click(timeline);
  });

  expect(
    await within(entitiesContainer!).findByText('Test Entity 5 (0)')
  ).toBeInTheDocument();
});

test('sessions get persisted to the database on socket disconnect', async () => {
  const { getByText, findByText } = await setupWithMessagesFromServer(
    stubMessages
  );

  WS.clean();

  const sessionContainer = (await findByText(/Previous sessions/i))
    .parentElement;
  const session = (
    await within(sessionContainer!).findAllByText(DATE_REGEX)
  )[0];

  act(() => {
    userEvent.click(session);
  });

  // Ensure the entities have loaded.
  const entitiesContainer = getByText(/Entities/i).parentElement;
  expect(
    await within(entitiesContainer!).findByText('Test Entity 4 (0)')
  ).toBeInTheDocument();
});

test('the app will attempt to reconnect', async () => {
  const { server } = await setupWithMessagesFromServer(stubMessages);

  act(() => {
    server.close();
  });
  await server.closed;
  WS.clean();

  const newServer = await setupWithWebSocket();
  await newServer.connected;
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
    await within(entitiesContainer!).findByText('Test Entity 1 (0)')
  ).toBeInTheDocument();
  expect(
    await within(entitiesContainer!).findByText('Test Entity 2 (1)')
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
  const entity = await within(entitiesContainer!).findByText(
    'Test Entity 1 (0)'
  );

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
  await within(entitiesContainer!).findByText('Test Entity 2 (1)');

  // Now, click on the scrubber so it loads the next frame..
  const timeline = getByTestId('scrubber');
  act(() => {
    userEvent.click(timeline);
  });

  const entity = await within(entitiesContainer!).findByText(
    'Test Entity 3 (0)'
  );
  expect(entity).toBeInTheDocument();
});
