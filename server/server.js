const path = require('path');

const express = require('express');
const bodyParser = require('body-parser');
const app = express();
const hri = require('human-readable-ids').hri;

const channelManager = require('./channel-manager');
const ParsedEvent = require('./parsers/ParsedEvent');

const MAX_SUBSCRIBERS_PER_STREAM = 30;
const MIN_BUCKET_ID_LENGTH = 10;

process.on('uncaughtException', (err) => console.log('Unhandled exception: ' + err));
process.on('unhandledRejection', (err) => console.log('Unhandled promise rejection: ' + err));
process.on('warning', (err) => console.log('Process warning: ' + err));

app.use((req, resp, next) => {
  // If the request does not contain a content type, assume it is text/plain
  if (req.method === 'POST' && !req.headers['content-type']) {
    req.headers['content-type'] = 'text/plain';
  }
  resp.set({
    'Access-Control-Allow-Origin': req.headers['origin'] || '*',
		'X-Accel-Buffering': 'no' // Disables response buffering on Google App Engine (flex env)
  });
  next();
});
app.use(express.static(path.join(__dirname, '../client/public')));
app.use(bodyParser.text({ type: "*/*", limit: '1mb' }));

app.set('views', './views');

app.disable('x-powered-by');
app.disable('etag');

// Allow this tool to be a log destination for Fastly's real time logging feature
app.all('/.well-known/fastly/logging/challenge', (req, res) => res.end("*"));

// Provide a healthcheck endpoint for Google App Engine
app.get('/__health', (req, res) => res.end('OK'));

// Redirect the root path to a random bucket
app.get('/', (req, res) => res.redirect('/' + hri.random()));

// Render a viewer UI or deliver an event stream for a bucket
app.get('/:bucketID', (req, res) => {
  if (req.params.bucketID.length < MIN_BUCKET_ID_LENGTH) {
    res.status(404);
    res.end('Bucket ID too short.  Minimum length: ' + MIN_BUCKET_ID_LENGTH);
    return;
  }
  if (req.headers['accept'] === 'text/event-stream') {
    const maxSubs = (req.params.bucketID.match(/;max-subs=(\d+)/) || [])[1] || MAX_SUBSCRIBERS_PER_STREAM;
    const ch = channelManager.getChannel(req.params.bucketID);
    if (ch.getSubscriberCount() >= maxSubs) {
      res.status(429);
      res.end('This stream already has the maximum number of clients connected');
    } else {
      console.log({ msg: "New subscriber", bucket: req.params.bucketID })
      ch.subscribe(req, res);
      const clients = Object.keys(ch.listClients());
      const connCount = ch.getSubscriberCount();
      ch.publish({ clientCount: clients.length, connCount, clients }, 'stats');
    }
  } else {
    res.sendFile(path.join(__dirname, '../client/index.html'));
  }
});

// Process new events
app.post('/:bucketID', (req, res) => {
  const lines = req.body && req.body.split(/\n+/).filter(x => x && x.length)
  if (lines) {
    console.log({ msg: "New events", bucket: req.params.bucketID, eventCount: lines.length })
    lines.forEach(line => {
      const evt = new ParsedEvent(line);
      evt.parse();

      channelManager.getChannel(req.params.bucketID).publish({
        time: evt.getTime(),
        raw: line,
        fields: evt.getFields(),
        parser: evt.getParserName()
      }, 'log');

    });
    res.status(204);
  } else {
    res.status(400);
    res.end("No content received")
  }
  res.end();
});

app.listen(process.env.PORT, () => console.log('Server up'));
