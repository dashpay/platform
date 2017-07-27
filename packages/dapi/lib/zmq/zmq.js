const zmq = require('zmq');

class ZMQ {
    constructor(app) {
        if (!app.hasOwnProperty('config') || !app.config.hasOwnProperty('zmq')) {
            throw new Error('Missing config for zmq.');
        }
        if ((app.config.zmq.hasOwnProperty('enable') && app.config.zmq.enable === false)) {
            app.zmq = this;
            return false;
        }
        this.setup(app);
    }
    setup(app) {
        const zmqSubSocket = zmq.socket('sub');
        zmqSubSocket.on('bind', function(fd, ep) { console.log('bind, endpoint:', ep); });
        zmqSubSocket.on('unbind', function(fd, ep) { console.log('unbind, endpoint:', ep); });
        zmqSubSocket.on('error', function(fd, ep) { console.log('error, endpoint:', ep); });
        zmqSubSocket.on('connect', function(fd, ep) { console.log('connect, endpoint:', ep); });
        zmqSubSocket.on('connect_delay', function(fd, ep) { console.log('connect_delay, endpoint:', ep); });
        zmqSubSocket.on('connect_retry', function(fd, ep) { console.log('connect_retry, endpoint:', ep); });
        zmqSubSocket.on('listen', function(fd, ep) { console.log('listen, endpoint:', ep); });
        zmqSubSocket.on('bind_error', function(fd, ep) { console.log('bind_error, endpoint:', ep); });
        zmqSubSocket.on('accept', function(fd, ep) { console.log('accept, endpoint:', ep); });
        zmqSubSocket.on('accept_error', function(fd, ep) { console.log('accept_error, endpoint:', ep); });
        zmqSubSocket.on('close', function(fd, ep) { console.log('close, endpoint:', ep); });
        zmqSubSocket.on('close_error', function(fd, ep) { console.log('close_error, endpoint:', ep); });
        zmqSubSocket.on('disconnect', function(fd, ep) { console.log('disconnect, endpoint:', ep); });
        // Handle monitor error
        zmqSubSocket.on('monitor_error', function(err) {
            console.log('Error in monitoring: %s, will restart monitoring in 5 seconds', err);
            setTimeout(function() { zmqSubSocket.monitor(500, 0); }, 5000);
        });
        console.log('Start monitoring...');
        zmqSubSocket.monitor(500, 0);
        zmqSubSocket.connect(app.config.zmq.url);
        zmqSubSocket.subscribe('hashblock');
        zmqSubSocket.subscribe('hashtx');
        zmqSubSocket.subscribe('rawblock');
        zmqSubSocket.subscribe('rawtx');
        zmqSubSocket.subscribe('rawtxlock');
        zmqSubSocket.on('message', function(topic, message) {
            var topicStr = topic.toString('utf8');
            switch (topicStr) {
                case "hashblock":
                    console.log('- HASH BLOCK - received')
                    // console.log(message.toString('hex'));
                    app.rpc.getBlockHeader(message.toString('hex')).then(function(res) {
                        console.log(res);
                    })
                    break;
                case "hashtx":
                    console.log('- HASH TX -', message.toString('hex'));
                    break;
                case "rawblock":
                    console.log('- RAW BLOCK HEADER received-', message.toString('hex'));
                    break;
                case "rawtx":
                    console.log('- RAW TX - - received');
                    // console.log(message.toString('hex'));
                    break;
                default:
                    console.log('unhandled ' + topicStr)
            }
        });
    }
}
module.exports = ZMQ;