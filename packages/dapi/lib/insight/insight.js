const request = require('request');
let URIS = {
	//FIXME : For now we just use an external insight, later on, we use embedded one
	testnet: 'https://test.insight.dash.siampm.com/api',
	livenet: 'https://insight.dash.siampm.com/api'
};
class Insight {
	constructor(app) {
		this.URI = (app.config.livenet) ? URIS['livenet'] : URIS['testnet'];
	}
	performGETRequest(path, req, res) {
		path = this.URI+path;
		req.pipe(request(path)).pipe(res);
		//TODO isvalidPath	
	}
	performPOSTRequest(path, data, req, res){
		path = this.URI+path;
		req.pipe(request.post({url:path, form:data}), {end: false}).pipe(res);
	}
};
module.exports = Insight;