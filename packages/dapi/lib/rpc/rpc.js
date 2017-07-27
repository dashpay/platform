const _ = require('lodash');
const Promise = require('bluebird');
const RpcClient = require('node-json-rpc2').Client;
const methodList = require('./methods');
class RPC {
    constructor(app) {
        if (!app.hasOwnProperty('config') || !app.config.hasOwnProperty('rpc')) {
            throw new Error('Missing config for rpc.');
        }
        app.rpc = this;
        let self = this;
        this.rpcClient = new RpcClient(app.config.rpc);
        _.each(methodList, function(expectedValue, methodName) {
            var params = [];
            if (expectedValue.length > 0) {
                params = expectedValue.split(' ');
            }
            if (!app.hasOwnProperty('config') || !app.config.hasOwnProperty('rpc') || (app.config.rpc.hasOwnProperty('enable') && app.config.rpc.enable === false)) {
                self[methodName] = function() { return new Promise(function(resolve, reject) { return reject('RPC Shutted down.') }) };
            } else {
                self[methodName] = self.createMethod(methodName.toLowerCase(), params)
            }
        })
    }
    createMethod(methodName, params) {
        let self = this;
        return function() {
            let argList = (arguments) ? Array.prototype.slice.call(arguments, 0) : [];
            // if(params.length<argList.length)
            //     throw new Error('Unexpected length');
            _.each(params, function(el, i) {
                if (typeof argList[i] !== params[i]) {
                    if (typeof argList[i] == 'undefined') {
                        throw new Error('Provide arguments ' + i + ' of type "' + params[i] + '"');
                    } else {
                        throw new Error('Expected arguments N°' + i + ' being type "' + params[i] + '" received type:' + typeof argList[i]);
                    }
                }
            });
            return new Promise(function(resolve, reject) {
                self.rpcClient.call({
                    method: methodName,
                    params: argList
                }, function(err, res) {
                    if (err) {
                        return resolve(err)
                    }
                    return resolve(res);
                });
            });
        }
    }

}
module.exports = RPC;