import moment from 'moment';

const readyStates = {
  0: 'connecting',
  1: 'open',
  2: 'closed'
}

class Stream {

  constructor (bucketID, options) {
    this.state = 2;
    this.stream = null;
    this.bucketID = bucketID;
    this.options = Object.assign({
      timeKeys: ['time', 'datetime', 'timestamp', 'logtime', 'eventtime', 'datestamp'],
      msgKeys: ['msg', 'message', ''],
      metaKeys: []
    }, options);
    this.handlers = { log: new Set(), stats: new Set(), stateChange: new Set() }
  }

  connect() {
    if (this.stream) this.stream.close();
    this.stream = new EventSource(`${location.origin}/${this.bucketID}`);
    this.stream.addEventListener('open', () => this.setConnectionState());
    this.stream.addEventListener('error', () => this.setConnectionState());
    this.stream.addEventListener('stats', e => {
      this.setConnectionState();
      this.emit('stats', JSON.parse(e.data))
    });
    this.stream.addEventListener('log', e => {
      this.setConnectionState();
      const data = JSON.parse(e.data);
      const fieldData = Object.assign({}, data.fields);
      const event = { raw: data.raw, time: data.time };

      // Parse time field or use current time
      const timeKey = this.options.timeKeys.find(k => k in fieldData);
      if (timeKey) {
        let timeVal = fieldData[timeKey].value;
        if (timeVal < 100000000000) { // A time that is too small to be in milliseconds
          timeVal *= 1000; // Convert seconds to milliseconds
        }
        event.timeString = moment(timeVal).format('HH:mm:ss');
        delete fieldData[timeKey];
      } else {
        event.timeString = moment(data.time).format('HH:mm:ss');
      }

      // Parse primary log message field
      const msgKey = this.options.msgKeys.find(k => k in fieldData);
      if (msgKey) {
        event.message = fieldData[msgKey].value;
        delete fieldData[msgKey];
      } else if (!Object.keys(fieldData).length) {
        event.message = data.raw;
      } else {
        event.message = null;
      }

      // If metaKeys is provided, filter out any unwanted fields
      event.fields = Object.keys(fieldData).reduce((out, key) => {
        if (!this.options.metaKeys || this.options.metaKeys.length === 0 || this.options.metaKeys.includes(key)) {
            return Object.assign(out, {[key]: fieldData[key]});
        } else {
            return out;
        }
      }, {});

      this.emit('log', event);
    });
  }

  setConnectionState() {
    if (this.state !== this.stream.readyState){
      this.state = this.stream.readyState;
      this.emit('stateChange', readyStates[this.state]);
    }
  }

  on (eventName, fn) {
    if (!(eventName in this.handlers)) throw new Error ('No such handler: ' + eventName);
    this.handlers[eventName].add(fn);
  }

  emit (eventName, data) {
    this.handlers[eventName].forEach(fn => fn.call(null, data));
  }
}

export default Stream;
