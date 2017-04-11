const {cl} = require('khal');
const config = require('../config.js');
const Buffer = require('./Buffer.js');
const PORT = config.ROUTER.port || 8080;
const HOST = config.ROUTER.host || '192.168.0.10';
const ws = require('ws');
const Connector = {
    createSocket:function(requester){
        return new Promise(function(resolve, reject){
            if(requester && requester.hasOwnProperty('CONNECTOR_TYPE')) {
                let type = requester.CONNECTOR_TYPE;
                if (type == "SERVER") {
                    requester.socket = new ws.Server({host: HOST, port: PORT});
                    // console.log('Serving...');
                    return resolve(true);
                }
                else if (type == "CLIENT") {
                    requester.socket = new ws('ws://' + HOST+ ':' + PORT);
                    requester.socket.on('open',function(){
                        // console.log('Connected...\n');
                        return resolve(true);
                    });
                    requester.socket.on('error',function(){
                        // console.log('Can\' connect');
                        return resolve(false);
                    })
                }else{
                    return reject(new Error('Unabled to create socket. Wrong type',type))
                }
            }else{
                return reject(new Error('Unabled to create socket. No type assigned'))
            }
        });
    },
    retryConnect:function(requester){
        if(requester && requester.hasOwnProperty('type') && requester.hasOwnProperty('socket')){
            requester.socket.on('error',function(){
                // cl('Error',{arguments});
            });
            requester.socket.on('close',function(statusCode,reason){
                // console.log('Closed by server',{arguments});

                let tryConnect = function(){
                    if(requester.socket.readyState==3) {
                        // console.log('Reconnecting...');
                        Connector.createSocket(requester);
                        Connector.retryConnect(requester);
                    }
                };
                let checkInterval = setInterval(function(){
                    if(requester.socket.readyState==3){
                        tryConnect();
                    }else{
                        clearInterval(checkInterval);
                    }
                },1000);
                tryConnect();
            })


        }
    }
};
module.exports= Connector;