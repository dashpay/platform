/*
 * pool.js - DAPI Pool Class
 * Manage the pool of peers
 */
const _ = require('lodash');
const Peer = require('./peer');
const emitter = require('./eventBus');
class Pool {
    constructor(config) {
        this.peers = {
            list: [],
            inbound: [],
            outbound: []
        }
        this.config = config;
        this.constructKnownPeers();
        this.handleNewPeerAnnounced();
        this.handleNewListAnnounced();
    }
    getList() {
        let cleanedList = [];
        for (let i = 0; i < Object.keys(this.peers.list).length; i++) {
            let _peer = this.peers.list[Object.keys(this.peers.list)[i]];
            cleanedList.push(
                {
                    pubKey: _peer.pubKey,
                    pub: {
                        host: _peer.pub.host,
                        port: _peer.pub.port
                    },
                    rep: {
                        host: _peer.rep.host,
                        port: _peer.rep.port
                    }
                });
        }
        return cleanedList;
    }
    constructKnownPeers() {
        const knownPeers = [
            {
                pubKey: "XkifrWK9yHVzXLgeAaqjhjDJuFad6b40000",
                rep: { uri: '127.0.0.1:40000' },
                pub: { uri: '127.0.0.1:50000' }
            }
            // '173.212.223.26:40000'
        ];
        for (let i = 0; i < knownPeers.length; i++) {
            let p = knownPeers[i];
            this.addPeer(new Peer({ pubKey: p.pubKey, rep: p.rep, pub: p.pub }));
        }
    }

    isKnownPeer(peer) {
        if (!(peer instanceof Peer)) {
            throw new Error('Trying to check if non peer is known.');
        }
        let known = false;
        for (let i = 0; i < Object.keys(this.peers.list).length; i++) {
            let _peer = this.peers.list[Object.keys(this.peers.list)[i]];
            if (peer.pubKey === _peer.pubKey &&
                peer.pub.host === _peer.pub.host &&
                peer.rep.host === _peer.rep.host &&
                peer.pub.port === _peer.pub.port &&
                peer.rep.port === _peer.rep.port) {
                known = true;
                break;
            }
        }
        return known;
    }

    /* Verify if a peer is a legit peer : 
     A peer should have a pubKey, is this a valid pubKey ?
     A peer should be pingable, is this the case ?
     */
    isValidPeer(peer) {
        return true;
    }

    addPeer(peer) {
        if (!(peer instanceof Peer)) {
            throw new Error('Trying to add non peer.');
        }
        if (!this.isValidPeer(peer)) {
            return false;
        }

        console.log(`Adding new peer ${peer.pubKey} in the list (${peer.rep.host} | [${peer.rep.port},${peer.pub.port}]`);
        this.peers.list.push(peer);
        //Because we found a new peer, we need to inform other node of that
        console.log(`Announcing peer ${peer.pubKey}`);
        emitter.emit('peer.announceNew', {
            pubKey: peer.pubKey,
            rep: {
                host: peer.rep.host,
                port: peer.rep.port
            },
            pub: {
                host: peer.pub.host,
                port: peer.pub.port
            }
        });

        peer.connect();
        //We advertise ourself to the peer we've connected to.
        peer.sendIdentity({
            pubKey: this.config.pubKey,
            pub: this.config.pub,
            rep: this.config.rep
        })
        //We ask for a peer list.
        peer.askPeerList();
    }
    handleNewListAnnounced() {
        let self = this;
        emitter.on('peerList.received', function(_list) {
            console.log(_list);
        })
    }

    handleNewPeerAnnounced() {
        let self = this;
        emitter.on('peer.receivedNew', function(_peer) {
            if (_peer.hasOwnProperty('pubKey') && _peer.hasOwnProperty('rep') && _peer.hasOwnProperty('pub')) {
                let peer = new Peer({
                    pubKey: _peer.pubKey,
                    pub: _peer.pub,
                    rep: _peer.rep
                });
                if (!self.isKnownPeer(peer)) self.addPeer(peer);
            }
        })
    }
}
module.exports = Pool;