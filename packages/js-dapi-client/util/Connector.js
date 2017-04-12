const {cl} = require('khal');
const config = require('../config.js');
const Buffer = require('./Buffer.js');
const ws = require('ws');
const Connector = {
    createSocket:function(requester){
        return new Promise(function(resolve, reject){
            if(requester && requester.hasOwnProperty('CONNECTOR_TYPE')) {
                let type = requester.CONNECTOR_TYPE;
                let PATH = requester.CONNECTOR_PATH;
                let PORT = requester.CONNECTOR_PORT;
                let HOST = requester.CONNECTOR_HOST;
                if (type == "SERVER") {
                    requester.socket = new ws.Server({host: HOST, port: PORT});
                    // console.log('Serving...');
                    return resolve(true);
                }
                else if (type == "CLIENT") {
                    if(PATH || (PORT && HOST)) {
                        // requester.socket = (PATH) ? new ws(PATH):new ws('ws://' + HOST + ':' + PORT);
                        // requester.socket = new ws('wss://insight.dash.siampm.com/socket.io/?EIO=2&transport=websocket&sid=Jl5OdlaoJjzkBTtRAACI');
                        requester.socket = new ws('https://insight.dash.siampm.com/socket.io/?EIO=2&transport=polling&t=1491974880972-0');
                        requester.socket.on('open',function(){
                            // console.log('Connected...\n');
                            return resolve(true);
                        });
                        requester.socket.on('error',function(){
                            console.log('Can\' connect');
                            return resolve(false);
                        })
                    }else{
                        console.log('Missing element to perform a socket client','HOST:',HOST,'-PORT:',PORT);
                        return resolve(false);
                    }
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