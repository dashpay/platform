/*
 * net.js - Networking backbone of DAPI 
 * Use zmq(0mq) to enable connecting.
 * 
 */

const _ = require('lodash');
const publisher = require('./publisher');
const subscriber = require('./subscriber');
const replier = require('./replier');
const requester = require('./requester');
const pairRequester = require('./pairRequester');
const pairResponder = require('./pairResponder');
const NET_TYPES = {
    "publisher": publisher,
    "pub": publisher,
    "subscriber": subscriber,
    "sub": subscriber,
    "replier": replier,
    "rep": replier,
    "requester": requester,
    "req": requester,
    "pairRequester": pairRequester,
    "pairResponder": pairResponder
};

class Net {
    constructor() {
    }

    /**
     *
     * @param params - an object containing theses data :
     *          type (required) : A valid DAPi networking-type being a string of either (sub:subscriber, pub:publisher, req:requester, rep:replier, pair:pairer)
     *          addr (required) : The address to which you want to sub/pub or whatever
     *
     */
    attach(params) {
        let self = this;

        function validAddr(uri) {
            return (uri && uri.constructor.name == "String" && uri.indexOf(':') > 0)
        }

        if (!_.has(params, 'type') || !NET_TYPES[params.type]) {
            throw new Error(`Not supported type. Valid type are : \n\t    ${Object.keys(NET_TYPES)}`);
        }
        if (!_.has(params, 'uri') || !validAddr(params['uri'])) {
            throw new Error(`Invalid or missing params uri - Received : ${params['uri']}`);
        }
        try {
            let uri = params['uri'];
            let hostname = uri.split(':')[0];
            let port = uri.split(':')[1];

            let sockParams = { uri: params['uri'] };
            if (_.get(params, 'onMessage')) sockParams.onMessage = params['onMessage'];

            let socket = new NET_TYPES[params.type](sockParams);
            socket.attach();
            return socket;


        } catch (e) {
            console.error(e);
            return false;
        }

    }

    detach(sock) {
        if (sock && _.has(sock, 'type') && _.has(sock, '_zmq')) {
            sock.close();
        }
    }
}
;
module.exports = Net;