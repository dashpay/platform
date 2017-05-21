const _ = require('lodash');
const Promise = require('bluebird');
const RpcClient = require('node-json-rpc2').Client;
const methodList = require('./methods');
class RPC {
    constructor(app){
        app.rpc = this;
        let self = this;
        this.rpcClient = new RpcClient(app.config.rpc);
        _.each(methodList, function(expectedValue, methodName){
            var params = [];
            if(expectedValue.length>0){
                params = expectedValue.split(' ');
            }
            self[methodName] = self.createMethod(methodName.toLowerCase(),params)
        })
    }
    createMethod(methodName, params) {
        let self = this;
        let argList = (arguments) ? Array.prototype.slice.call(arguments, 0) : [];
        _.each(params, function(el,i) {
            if (argList[1][i] !== params[i]) {
                if (typeof argList[1][i] == 'undefined') {
                    throw new Error('Provide arguments ' + i + ' of type "' + params[i] + '"');
                } else {
                    throw new Error('Expected arguments N°' + i + ' being type "' + params[i] + '" received type:' + typeof argList[1][i]);
                }
            }
        });
        return function(){
            return new Promise(function (resolve, reject) {
                self.rpcClient.call({
                    method: methodName,
                    params: argList
                },function(err, res){
                    if (err) {
                        console.log(err,res);
                        return resolve({})
                    }
                    return resolve(res);
                });
            });
        }
        
    }
}
module.exports = RPC;