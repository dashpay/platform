const request = require('request-promise-native');
const mockListGenerator = require('../mocks/dynamicMnList')

let URIS = {
	//FIXME : For now we just use an external insight, later on, we use embedded one
	testnet: 'http://dev-test.insight.dashevo.org/insight-api-dash',
	livenet: 'http://insight.dashevo.org/insight-api-dash'
}

class Insight {

	constructor(app) {
		this.URI = (app.config.livenet) ? URIS['livenet'] : URIS['testnet']
		this.mnListGenerator = new mockListGenerator()
	}

	performGETRequest(path, req, res) {
		path = this.URI + path
		req.pipe(request(path)).pipe(res)
		req.headers['x-forwarded-for'] = req.ip;
		//TODO isvalidPath	
	}

	performPOSTRequest(path, data, req, res) {
		path = this.URI + path;
		req.pipe(request.post({ url: path, form: data }), { end: false }).pipe(res)
		req.headers['x-forwarded-for'] = req.ip;
	}

	getAddress(txHash) {
		let uri = this.URI;
		return new Promise(function(resolve, reject) {
			request(uri + '/tx/' + txHash, function(err, response, body) {
				resolve(JSON.parse(body).vin[0].addr)
			})
		})
	}

	getCurrentBlockHeight() {
		let uri = this.URI;
		return new Promise(function(resolve, reject) {
			request(uri + '/status', function(err, response, body) {
				resolve(JSON.parse(body).info.blocks)
			})
		})
	}

	getHashFromHeight(height) {
		let uri = this.URI;
		return new Promise(function(resolve, reject) {
			request(uri + `/block-index/${height}`, function(err, response, body) {
				resolve(JSON.parse(body).blockHash)
			})
		})
	}

	getMnList() {
		return this.mnListGenerator.getMockMnList()
	}

	getMnUpdateList(hash) {
		return this.mnListGenerator.getMockMnUpdateList()
	}
}

module.exports = Insight;
