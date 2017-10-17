const request = require('request');
let URIS = {
	//FIXME : For now we just use an external insight, later on, we use embedded one
	testnet: 'http://dev-test.insight.dashevo.org/insight-api-dash',
	livenet: 'http://insight.dashevo.org/insight-api-dash'
};
class Insight {
	constructor(app) {
		this.URI = (app.config.livenet) ? URIS['livenet'] : URIS['testnet'];
	}
	performGETRequest(path, req, res) {
		path = this.URI + path;
		req.pipe(request(path)).pipe(res);
		//TODO isvalidPath	
	}
	performPOSTRequest(path, data, req, res) {
		path = this.URI + path;
		req.pipe(request.post({ url: path, form: data }), { end: false }).pipe(res);
	}

	//Todo: Temp to get last block hash for quorums //////////////////////////////////////////

	getLastBlockHash() {

		const refHeight = 100; //todo: move to config
		let uri = this.URI;
		return new Promise(function(resolve, reject) {
			request(uri + '/status', function(err, response, body) {
				if (err) reject(err);
				request(uri + `/block-index/${JSON.parse(body).info.blocks - refHeight}`, function(err, response, body) {
					if (err) reject(err);
					resolve(JSON.parse(body).blockHash);
				});
			});
		});

	}

	getMnList() {
		let uri = this.URI;
		return new Promise(function(resolve, reject) {
			request(uri + '/masternodes/list/', function(err, response, body) {
				if (err) reject(err);
				resolve([JSON.parse(body)[0]]); //todo: temp only return first localhost server for dev purposes
			});
		});
	}

	getAddress(txHash) {
		let uri = this.URI;
		return new Promise(function(resolve, reject) {
			request(uri + '/tx/' + txHash, function(err, response, body) {
				resolve(JSON.parse(body).vin[0].addr);
			});
		});
	}

	//Temp insight quorums end /////////////////////////////////////////////////////////////////
};
module.exports = Insight;