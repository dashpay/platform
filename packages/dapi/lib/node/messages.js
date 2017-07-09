/*
 * messages.js - DAPI Messages Class
 * Manage a message
 */
const defaultMessages = {
    'ping':{type:'ping'},
    'pong':{type:'pong'},
    'ack':{type:'ack'},
    'identity':{type:'identity'},
    'newPeer':{type:'newPeer'},
    'peerList':{type:'peerList'}
};
const _ = require('lodash');
const {uuid} = require('khal');
const extend = require('extend');
class Messages {
    constructor(msgType){
        if(!_.has(defaultMessages, msgType)){
            return new Error(`${msgType} is not an allowed message's type.`);
        }
        this.data = JSON.parse(JSON.stringify(defaultMessages[msgType]));
    }
    addData(data){
        extend(true,this.data, data);
    }
    addCorrelationId(){
        this.data.correlationId = this.data.hasOwnProperty('correlationId')? this.data.correlationId:uuid.generate.v4();
    }
    prepare(){
        return JSON.stringify(this.data);
    }
}
module.exports=Messages;