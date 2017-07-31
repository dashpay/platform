/*
 * node.js - DAPI node class
 * 
 */
const Net = require('../net/net');
const Pool = require('../node/pool');
const Peer = require('../node/peer');
const kvs = require('../mempool/keyValueStore');
const { cl } = require('khal');
const _ = require('lodash');
const Messages = require('../node/messages');
const emitter = require('./eventBus');

class Node {
    constructor(params) {
        this.config = {
            pubKey: _.get(params, 'pubKey') || 'InvalidPubKey',
            privKey: _.get(params, 'privKey') || 'InvalidPubKey',
            rep: {
                uri: '127.0.0.1:' + (_.get(params, 'rep.port') || '40000')
            },
            pub: {
                uri: '127.0.0.1:' + (_.get(params, 'pub.port') || '50000')
            }
        };
        cl('Config :', this.config);

        this.socks = {};
        this.mempool = new kvs(_.get(params, 'orbitPort'))
        this.init();

        let self = this;
        setTimeout(function() {
            self.start();
        }, 1000)//We set this with a delay to let time for our node to bound. Later we will remove that : FIXME.
    }

    init() {
        cl('Initializing...');
        const net = new Net();
        //setup replier on the port 40000. 
        //Replier uses are : 
        // - Allow for a new node to inform us that they exist.
        this.socks.rep = net.attach({
            type: 'rep',
            uri: this.config.rep.uri,
            onMessage: onReplierMessage.bind(this)
        });

        //setup publisher on the port 50000.
        //Publisher uses are : 
        // - Inform all subscriber that a new data has came into our knowledge
        this.socks.pub = net.attach({
            type: 'pub',
            uri: this.config.pub.uri,
            onMessage: onPublisherMessage.bind(this)
        });
    }

    start() {
        cl(`Started node \n`);
        this.pool = new Pool(this.config);
        this.announceNewPeer();
    }

    announceNewPeer() {
        let self = this;
        emitter.on('peer.announceNew', function(peer) {
            if (peer.hasOwnProperty("pubKey") && peer.hasOwnProperty('rep') && peer.hasOwnProperty('pub')) {
                let newPeerMsg = new Messages('newPeer');
                newPeerMsg.addData({ peer: peer });
                console.log('SEND PUB', newPeerMsg.prepare());
                self.socks.pub.socket.send(newPeerMsg.prepare());
            }
        })

    }

    stop() {

    }

    addMemPoolData(mnPrivKey, mnPubAdr, value, key) {
        if (this.isUniqueKey(key)) {
            this.mempool.writeValue(mnPrivKey, mnPubAdr, value, key);
        }
    }

    getMemPoolData(key) {
        return this.mempool.getValue(key);
    }

    isUniqueKey(key) {
        return !this.mempool.getValue(key);
    }
}

function onPublisherMessage(msg) {
    let self = this;
    console.log('----- PUBLISHER received :')
    console.log(msg);
    console.log('----- /pub:')
}

function onReplierMessage(msg) {
    let self = this;
    if (msg && msg.hasOwnProperty('type')) {
        switch (msg.type) {
            case "ping":
                let pong = new Messages('pong');
                pong.addData({ correlationId: msg.correlationId });
                this.socks.rep.socket.send(pong.prepare());
                break;
            case "peerList":
                let cleanedList = self.pool.getList();
                let peerList = new Messages('peerList');
                peerList.addData({ list: cleanedList });
                self.socks.rep.socket.send(peerList.prepare());
                break;
            case "identity":
                if (msg.hasOwnProperty('pubKey') && msg.hasOwnProperty('rep') && msg.hasOwnProperty('pub')) {
                    let peer = new Peer({
                        pubKey: msg.pubKey,
                        pub: msg.pub,
                        rep: msg.rep
                    })
                    if (!self.pool.isKnownPeer(peer)) self.pool.addPeer(peer);
                }
                this.socks.rep.socket.send(new Messages('ack').prepare());
                break;
            default:
                this.socks.rep.socket.send(new Messages('ack').prepare());
                break;
        }
    }
}

module.exports = Node;