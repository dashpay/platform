const _ = require('lodash');
const zmq = require('zmq');
const Messages = require('../node/messages');

function defaultOnMessage(msg) {//pvr: this probably needs to be futher split into the send/receive base socket classes
    console.log(`${this.constructor.name} - Received message ${msg}`);
    this.socket.send(new Messages('ack').prepare());
}

class SocketBase {
    constructor(params) {
        this.uri = (params.uri.indexOf('tcp://') == -1) ? `tcp://${params.uri}` : params.uri;
        this.onMessage = params.hasOwnProperty('onMessage') ? params.onMessage : defaultOnMessage.bind(this);
    }

    attach(socketType) {

        this.socket = zmq.socket(socketType)

        this.socket.on('message', function(msg) {
            if (Buffer.isBuffer(msg)) msg = msg.toString();
            try { msg = JSON.parse(msg) } catch (e) { }
            self.onMessage(msg);
        });

        this.socket.monitor(500, 0)
        console.log(`${this.constructor.name} bound to ${this.uri}`)
    }

    detach() {
        if (this.socket) {
            this.socket.close();
        }
        return true;
    }
}

module.exports = SocketBase;