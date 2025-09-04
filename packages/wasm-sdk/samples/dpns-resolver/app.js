/**
 * DPNS Resolver - Dash Platform WASM SDK Sample Application
 * Demonstrates DPNS operations including username resolution, validation, and registration cost calculation
 */

import init, { WasmSdk, WasmSdkBuilder } from '../../pkg/wasm_sdk.js';
import { DPNSValidator } from './validation.js';

class DPNSResolver {
    constructor() {
        this.sdk = null;
        this.currentNetwork = 'testnet';
        this.isInitialized = false;
        this.validator = new DPNSValidator();
        this.dpnsContractId = null;
        this.debugMode = false;
        
        // Sample data for testing
        this.sampleUsername = 'alice';
        this.sampleIdentity = '5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk';
        
        // DPNS contract IDs
        this.dpnsContracts = {
            testnet: 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec',
            mainnet: '566vcJkmebVCAb2Dkj2yVMSgGFcsshupnQqtsz1RFbcy'
        };
        
        // Bind methods
        this.initializeSDK = this.initializeSDK.bind(this);
        this.resolveUsername = this.resolveUsername.bind(this);
        this.validateUsername = this.validateUsername.bind(this);
        this.reverseResolve = this.reverseResolve.bind(this);
        this.browseDomains = this.browseDomains.bind(this);
        this.calculateRegistrationCost = this.calculateRegistrationCost.bind(this);
        this.refreshStats = this.refreshStats.bind(this);
    }

    async init() {
        try {
            await this.initializeEventListeners();
            await this.initializeSDK();
            this.setupValidationRealTime();
            this.logOperation('System', 'DPNS Resolver initialized successfully');
        } catch (error) {
            this.showError('Failed to initialize application: ' + error.message);
            this.logOperation('System', `Initialization failed: ${error.message}`, 'error');
        }
    }

    initializeEventListeners() {
        // Network selection
        const networkSelect = document.getElementById('networkSelect');
        networkSelect.addEventListener('change', async (e) => {
            this.currentNetwork = e.target.value;
            this.dpnsContractId = this.dpnsContracts[this.currentNetwork];
            await this.initializeSDK();
        });

        // Username resolution
        const resolveBtn = document.getElementById('resolveBtn');
        resolveBtn.addEventListener('click', this.resolveUsername);

        // Username validation
        const validateBtn = document.getElementById('validateBtn');
        validateBtn.addEventListener('click', this.validateUsername);

        // Reverse resolution
        const reverseResolveDirectBtn = document.getElementById('reverseResolveDirectBtn');
        reverseResolveDirectBtn.addEventListener('click', this.reverseResolve);

        const reverseResolveBtn = document.getElementById('reverseResolveBtn');
        if (reverseResolveBtn) {
            reverseResolveBtn.addEventListener('click', this.reverseResolveFromResults.bind(this));
        }

        // Domain browsing
        const browseDomainBtn = document.getElementById('browseDomainBtn');
        browseDomainBtn.addEventListener('click', this.browseDomains);

        // Domain filter changes
        const domainFilter = document.getElementById('domainFilter');
        domainFilter.addEventListener('change', this.onDomainFilterChange.bind(this));

        // Registration cost calculation
        const calculateCostBtn = document.getElementById('calculateCostBtn');
        calculateCostBtn.addEventListener('click', this.calculateRegistrationCost);

        // Statistics
        const refreshStatsBtn = document.getElementById('refreshStatsBtn');
        refreshStatsBtn.addEventListener('click', this.refreshStats);

        // Enter key support
        const usernameInput = document.getElementById('usernameInput');
        usernameInput.addEventListener('keypress', (e) => {
            if (e.key === 'Enter') {
                this.resolveUsername();
            }
        });

        const validateUsernameInput = document.getElementById('validateUsernameInput');
        validateUsernameInput.addEventListener('keypress', (e) => {
            if (e.key === 'Enter') {
                this.validateUsername();
            }
        });

        const identityResolveInput = document.getElementById('identityResolveInput');
        identityResolveInput.addEventListener('keypress', (e) => {
            if (e.key === 'Enter') {
                this.reverseResolve();
            }
        });
    }

    setupValidationRealTime() {
        const usernameInput = document.getElementById('usernameInput');
        const validateUsernameInput = document.getElementById('validateUsernameInput');
        const registrationUsername = document.getElementById('registrationUsername');

        // Real-time validation for resolution input
        usernameInput.addEventListener('input', (e) => {
            this.showRealtimeValidation(e.target.value, 'validationInfo');
        });

        // Real-time validation for validation input
        validateUsernameInput.addEventListener('input', (e) => {
            this.showRealtimeValidation(e.target.value, 'validationInfo');
        });

        // Real-time cost calculation for registration input
        registrationUsername.addEventListener('input', (e) => {
            this.updateRegistrationFactors(e.target.value);
        });
    }

    async initializeSDK() {
        try {
            this.updateStatus('connecting', 'Initializing SDK...');
            this.logOperation('SDK', `Initializing for ${this.currentNetwork} network`);

            // Set DPNS contract ID for current network
            this.dpnsContractId = this.dpnsContracts[this.currentNetwork];

            // Initialize WASM module
            await init();

            // Create SDK builder based on network
            let builder;
            if (this.currentNetwork === 'mainnet') {
                builder = WasmSdkBuilder.new_mainnet_trusted();
            } else {
                builder = WasmSdkBuilder.new_testnet_trusted();
            }

            // Build SDK instance
            this.sdk = builder.build();
            this.isInitialized = true;

            this.updateStatus('connected', `Connected to ${this.currentNetwork}`);
            this.logOperation('SDK', `Successfully connected to ${this.currentNetwork}`, 'success');
            this.logOperation('DPNS', `Using contract ID: ${this.dpnsContractId}`);

        } catch (error) {
            this.updateStatus('error', 'Connection failed');
            this.logOperation('SDK', `Connection failed: ${error.message}`, 'error');
            throw error;
        }
    }

    async resolveUsername() {
        if (!this.isInitialized) {
            this.showError('SDK not initialized. Please wait for connection.');
            return;
        }

        const username = document.getElementById('usernameInput').value.trim().toLowerCase();

        if (!username) {
            this.showError('Please enter a username');
            return;
        }

        const validation = this.validator.validateAsYouType(username);
        if (!validation.valid) {
            this.showError(`Invalid username: ${validation.messages.join(', ')}`);
            return;
        }

        try {
            this.showLoadingOverlay('Resolving username...');
            this.logOperation('Resolution', `Resolving username: ${username}`);

            const startTime = Date.now();

            // Use DPNS resolution method
            const result = await this.sdk.dpns_resolve_name(username);
            
            const resolutionTime = Date.now() - startTime;

            if (result) {
                this.displayResolutionResults(username, result, resolutionTime);
                this.logOperation('Resolution', `Username ${username} resolved successfully in ${resolutionTime}ms`, 'success');
            } else {
                this.displayResolutionNotFound(username, resolutionTime);
                this.logOperation('Resolution', `Username ${username} not found`, 'error');
            }

        } catch (error) {
            this.showError('Failed to resolve username: ' + error.message);
            this.logOperation('Resolution', `Resolution failed for ${username}: ${error.message}`, 'error');
        } finally {
            this.hideLoadingOverlay();
        }
    }

    async validateUsername() {
        const username = document.getElementById('validateUsernameInput').value.trim().toLowerCase();

        if (!username) {
            this.showError('Please enter a username to validate');
            return;
        }

        try {
            this.showLoadingOverlay('Validating username...');
            this.logOperation('Validation', `Validating username: ${username}`);

            // Comprehensive validation
            const localValidation = this.validator.validateUsername(username);
            
            // Network-based validation
            const networkValidation = await this.performNetworkValidation(username);

            // Combine results
            const fullValidation = {
                ...localValidation,
                network: networkValidation,
                qualityScore: this.validator.calculateQualityScore(username),
                suggestions: localValidation.valid ? [] : this.validator.generateSuggestions(username)
            };

            this.displayValidationResults(fullValidation);
            this.logOperation('Validation', `Validation completed for ${username}`, 'success');

        } catch (error) {
            this.showError('Failed to validate username: ' + error.message);
            this.logOperation('Validation', `Validation failed for ${username}: ${error.message}`, 'error');
        } finally {
            this.hideLoadingOverlay();
        }
    }

    async performNetworkValidation(username) {
        const networkValidation = {
            available: null,
            contested: null,
            homographSafe: null
        };

        try {
            // Check if available
            if (document.getElementById('checkAvailability').checked) {
                const available = await this.sdk.dpns_is_name_available(username);
                networkValidation.available = available;
            }

            // Check if contested
            if (document.getElementById('checkContested').checked) {
                const contested = await this.sdk.dpns_is_contested_username(username);
                networkValidation.contested = contested;
            }

            // Check homograph safety
            if (document.getElementById('checkHomograph').checked) {
                const homographSafe = await this.sdk.dpns_convert_to_homograph_safe(username);
                networkValidation.homographSafe = homographSafe !== username ? homographSafe : true;
            }

        } catch (error) {
            this.logOperation('Validation', `Network validation error: ${error.message}`, 'debug');
        }

        return networkValidation;
    }

    async reverseResolve() {
        if (!this.isInitialized) {
            this.showError('SDK not initialized. Please wait for connection.');
            return;
        }

        const identityId = document.getElementById('identityResolveInput').value.trim();

        if (!identityId) {
            this.showError('Please enter an identity ID');
            return;
        }

        if (!this.isValidIdentityId(identityId)) {
            this.showError('Invalid identity ID format');
            return;
        }

        try {
            this.showLoadingOverlay('Finding usernames for identity...');
            this.logOperation('Reverse', `Reverse resolving identity: ${identityId}`);

            // Query domains owned by this identity
            const domains = await this.sdk.get_documents(
                this.dpnsContractId,
                'domain',
                JSON.stringify([['$ownerId', '=', identityId]]),
                JSON.stringify([['$createdAt', 'desc']]),
                50, // limit
                0   // offset
            );

            this.displayReverseResults(identityId, domains);
            this.logOperation('Reverse', `Found ${domains?.length || 0} domains for identity`, 'success');

        } catch (error) {
            this.showError('Failed to reverse resolve: ' + error.message);
            this.logOperation('Reverse', `Reverse resolution failed: ${error.message}`, 'error');
        } finally {
            this.hideLoadingOverlay();
        }
    }

    async browseDomains() {
        if (!this.isInitialized) {
            this.showError('SDK not initialized. Please wait for connection.');
            return;
        }

        try {
            const filter = document.getElementById('domainFilter').value;
            const limit = parseInt(document.getElementById('domainLimit').value) || 25;

            this.showLoadingOverlay('Loading domain registry...');
            this.logOperation('Browse', `Browsing domains with filter: ${filter}`);

            const query = this.buildDomainQuery(filter, limit);
            const startTime = Date.now();

            const domains = await this.sdk.get_documents(
                this.dpnsContractId,
                'domain',
                query.where,
                query.orderBy,
                query.limit,
                query.offset
            );

            const browseTime = Date.now() - startTime;
            this.displayBrowseResults(domains, browseTime, filter);
            this.logOperation('Browse', `Domain browse completed in ${browseTime}ms, found ${domains?.length || 0} domains`, 'success');

        } catch (error) {
            this.showError('Failed to browse domains: ' + error.message);
            this.logOperation('Browse', `Domain browse failed: ${error.message}`, 'error');
        } finally {
            this.hideLoadingOverlay();
        }
    }

    buildDomainQuery(filter, limit) {
        const query = {
            where: null,
            orderBy: JSON.stringify([['$createdAt', 'desc']]),
            limit,
            offset: 0
        };

        switch (filter) {
            case 'recent':
                // Recent registrations (default order)
                break;

            case 'short':
                // Short names (≤5 characters) - this would need a custom index
                // For now, we'll get recent and filter client-side
                break;

            case 'long':
                // Long names (>10 characters) - similar limitation
                break;

            case 'owner':
                const ownerId = document.getElementById('ownerFilter').value.trim();
                if (ownerId) {
                    query.where = JSON.stringify([['$ownerId', '=', ownerId]]);
                }
                break;
        }

        return query;
    }

    async calculateRegistrationCost() {
        const username = document.getElementById('registrationUsername').value.trim().toLowerCase();

        if (!username) {
            this.showError('Please enter a username');
            return;
        }

        try {
            this.logOperation('Cost', `Calculating registration cost for: ${username}`);

            const costEstimate = this.validator.estimateRegistrationCost(username);
            
            if (costEstimate.valid) {
                this.displayRegistrationCost(username, costEstimate);
                this.logOperation('Cost', `Cost calculated: ${costEstimate.cost} credits`, 'success');
            } else {
                this.showError(`Cannot calculate cost: ${costEstimate.errors.join(', ')}`);
                this.logOperation('Cost', `Cost calculation failed: invalid username`, 'error');
            }

        } catch (error) {
            this.showError('Failed to calculate cost: ' + error.message);
            this.logOperation('Cost', `Cost calculation failed: ${error.message}`, 'error');
        }
    }

    async refreshStats() {
        if (!this.isInitialized) {
            this.showError('SDK not initialized. Please wait for connection.');
            return;
        }

        try {
            this.showLoadingOverlay('Gathering DPNS statistics...');
            this.logOperation('Stats', 'Refreshing DPNS network statistics');

            // Get recent domains for statistics calculation
            const recentDomains = await this.sdk.get_documents(
                this.dpnsContractId,
                'domain',
                null,
                JSON.stringify([['$createdAt', 'desc']]),
                100, // Sample size for stats
                0
            );

            // Get domains from last 24 hours
            const oneDayAgo = Date.now() - (24 * 60 * 60 * 1000);
            const recentDomainsDay = await this.sdk.get_documents(
                this.dpnsContractId,
                'domain',
                JSON.stringify([['$createdAt', '>', oneDayAgo]]),
                null,
                100,
                0
            );

            const stats = this.calculateDomainStats(recentDomains, recentDomainsDay);
            this.displayStats(stats);
            this.logOperation('Stats', 'Statistics refreshed successfully', 'success');

        } catch (error) {
            this.showError('Failed to refresh statistics: ' + error.message);
            this.logOperation('Stats', `Statistics refresh failed: ${error.message}`, 'error');
        } finally {
            this.hideLoadingOverlay();
        }
    }

    // Display Methods

    displayResolutionResults(username, result, resolutionTime) {
        const resolutionResults = document.getElementById('resolutionResults');
        const resolvedData = document.getElementById('resolvedData');
        const resolutionStatus = document.getElementById('resolutionStatus');
        const resolutionTimeSpan = document.getElementById('resolutionTime');

        resolutionStatus.textContent = 'Found';
        resolutionStatus.className = 'status-badge';
        resolutionTimeSpan.textContent = `${resolutionTime}ms`;

        const displayData = {
            username: username + '.dash',
            resolvedTo: result,
            network: this.currentNetwork,
            contractId: this.dpnsContractId,
            resolvedAt: new Date().toISOString(),
            resolutionTime: resolutionTime
        };

        resolvedData.textContent = JSON.stringify(displayData, null, 2);
        resolutionResults.style.display = 'block';
        resolutionResults.scrollIntoView({ behavior: 'smooth', block: 'start' });

        // Store for reverse resolution
        this.lastResolution = { username, result };
    }

    displayResolutionNotFound(username, resolutionTime) {
        const resolutionResults = document.getElementById('resolutionResults');
        const resolvedData = document.getElementById('resolvedData');
        const resolutionStatus = document.getElementById('resolutionStatus');
        const resolutionTimeSpan = document.getElementById('resolutionTime');

        resolutionStatus.textContent = 'Not Found';
        resolutionStatus.className = 'status-badge not-found';
        resolutionTimeSpan.textContent = `${resolutionTime}ms`;

        const displayData = {
            username: username + '.dash',
            status: 'not_found',
            available: true,
            network: this.currentNetwork,
            searchedAt: new Date().toISOString(),
            resolutionTime: resolutionTime,
            suggestion: `This username appears to be available for registration on ${this.currentNetwork}`
        };

        resolvedData.textContent = JSON.stringify(displayData, null, 2);
        resolutionResults.style.display = 'block';
        resolutionResults.scrollIntoView({ behavior: 'smooth', block: 'start' });
    }

    displayValidationResults(validation) {
        const validationResults = document.getElementById('validationResults');
        const formatValid = document.getElementById('formatValid');
        const availabilityStatus = document.getElementById('availabilityStatus');
        const contestedStatus = document.getElementById('contestedStatus');
        const homographStatus = document.getElementById('homographStatus');
        const validationDetails = document.getElementById('validationDetails');

        // Update validation grid
        formatValid.textContent = validation.valid ? '✅ Valid' : '❌ Invalid';
        formatValid.className = `validation-value ${validation.valid ? 'valid' : 'invalid'}`;

        if (validation.network.available !== null) {
            availabilityStatus.textContent = validation.network.available ? '✅ Available' : '❌ Taken';
            availabilityStatus.className = `validation-value ${validation.network.available ? 'available' : 'unavailable'}`;
        }

        if (validation.network.contested !== null) {
            contestedStatus.textContent = validation.network.contested ? '⚠️ Contested' : '✅ Not Contested';
            contestedStatus.className = `validation-value ${validation.network.contested ? 'invalid' : 'valid'}`;
        }

        if (validation.network.homographSafe !== null) {
            const isSafe = validation.network.homographSafe === true;
            homographStatus.textContent = isSafe ? '✅ Safe' : '⚠️ Has Lookalikes';
            homographStatus.className = `validation-value ${isSafe ? 'valid' : 'invalid'}`;
        }

        // Update details
        const detailsData = {
            validation: validation,
            formatted: this.validator.formatValidationResults(validation),
            suggestions: validation.suggestions,
            qualityScore: `${validation.qualityScore}/100`,
            estimatedCost: this.validator.estimateRegistrationCost(validation.username)
        };

        validationDetails.textContent = JSON.stringify(detailsData, null, 2);
        validationResults.style.display = 'block';
        validationResults.scrollIntoView({ behavior: 'smooth', block: 'start' });
    }

    displayReverseResults(identityId, domains) {
        const reverseResults = document.getElementById('reverseResults');
        const usernameList = document.getElementById('usernameList');

        usernameList.innerHTML = '';

        if (!domains || domains.length === 0) {
            usernameList.innerHTML = `
                <div class="empty-state">
                    <div class="empty-icon">🌐</div>
                    <h4>No Usernames Found</h4>
                    <p>This identity doesn't own any registered usernames on ${this.currentNetwork}</p>
                </div>
            `;
        } else {
            domains.forEach(domain => {
                const usernameElement = this.createUsernameElement(domain);
                usernameList.appendChild(usernameElement);
            });
        }

        reverseResults.style.display = 'block';
        reverseResults.scrollIntoView({ behavior: 'smooth', block: 'start' });
    }

    createUsernameElement(domain) {
        const usernameDiv = document.createElement('div');
        usernameDiv.className = 'username-item';

        const nameSpan = document.createElement('span');
        nameSpan.className = 'username-name';
        nameSpan.textContent = (domain.data?.label || domain.label || 'Unknown') + '.dash';

        const actionsDiv = document.createElement('div');
        actionsDiv.className = 'username-actions';

        const resolveBtn = document.createElement('button');
        resolveBtn.className = 'btn btn-small btn-primary';
        resolveBtn.textContent = 'Resolve';
        resolveBtn.addEventListener('click', () => {
            document.getElementById('usernameInput').value = domain.data?.label || domain.label;
            this.resolveUsername();
        });

        const detailsBtn = document.createElement('button');
        detailsBtn.className = 'btn btn-small btn-secondary';
        detailsBtn.textContent = 'Details';
        detailsBtn.addEventListener('click', () => this.showDomainDetails(domain));

        actionsDiv.appendChild(resolveBtn);
        actionsDiv.appendChild(detailsBtn);

        usernameDiv.appendChild(nameSpan);
        usernameDiv.appendChild(actionsDiv);

        return usernameDiv;
    }

    displayBrowseResults(domains, browseTime, filter) {
        const browserResults = document.getElementById('browserResults');
        const domainGrid = document.getElementById('domainGrid');
        const domainCount = document.getElementById('domainCount');
        const browseTimeSpan = document.getElementById('browseTime');

        domainCount.textContent = `${domains?.length || 0} domains`;
        browseTimeSpan.textContent = `${browseTime}ms`;

        domainGrid.innerHTML = '';

        if (!domains || domains.length === 0) {
            domainGrid.innerHTML = `
                <div class="empty-state">
                    <div class="empty-icon">🌐</div>
                    <h4>No Domains Found</h4>
                    <p>No domains match the current filter criteria</p>
                </div>
            `;
        } else {
            // Apply client-side filtering for length-based filters
            let filteredDomains = domains;
            
            if (filter === 'short') {
                filteredDomains = domains.filter(d => (d.data?.label || d.label || '').length <= 5);
            } else if (filter === 'long') {
                filteredDomains = domains.filter(d => (d.data?.label || d.label || '').length > 10);
            }

            filteredDomains.forEach(domain => {
                const domainElement = this.createDomainElement(domain);
                domainGrid.appendChild(domainElement);
            });

            if (filteredDomains.length !== domains.length) {
                domainCount.textContent = `${filteredDomains.length} of ${domains.length} domains`;
            }
        }

        browserResults.style.display = 'block';
        browserResults.scrollIntoView({ behavior: 'smooth', block: 'start' });
    }

    createDomainElement(domain) {
        const domainDiv = document.createElement('div');
        domainDiv.className = 'domain-item';
        domainDiv.addEventListener('click', () => this.showDomainDetails(domain));

        const domainName = domain.data?.label || domain.label || 'Unknown';

        domainDiv.innerHTML = `
            <div class="domain-header">
                <span class="domain-name">${domainName}</span>
                <span class="domain-extension">.dash</span>
            </div>
            <div class="domain-meta">
                <span><strong>Length:</strong> ${domainName.length} chars</span>
                <span><strong>Created:</strong> ${this.formatDate(domain.createdAt || domain.$createdAt)}</span>
            </div>
            <div class="domain-owner">
                <strong>Owner:</strong> ${this.truncateId(domain.ownerId || domain.$ownerId || 'Unknown')}
            </div>
        `;

        return domainDiv;
    }

    displayRegistrationCost(username, costEstimate) {
        const registrationResults = document.getElementById('registrationResults');
        const costBreakdown = document.getElementById('costBreakdown');

        const costData = {
            username: username + '.dash',
            estimatedCost: {
                total: costEstimate.cost,
                baseCost: costEstimate.baseCost,
                lengthMultiplier: costEstimate.lengthMultiplier,
                inDash: (costEstimate.cost / 100000000).toFixed(8),
                inCredits: costEstimate.cost.toLocaleString()
            },
            factors: costEstimate.factors,
            qualityScore: this.validator.calculateQualityScore(username),
            network: this.currentNetwork,
            disclaimer: 'This is an estimate. Actual costs may vary based on network conditions.',
            calculatedAt: new Date().toISOString()
        };

        costBreakdown.textContent = JSON.stringify(costData, null, 2);
        registrationResults.style.display = 'block';
        registrationResults.scrollIntoView({ behavior: 'smooth', block: 'start' });
    }

    displayStats(stats) {
        const dpnsStats = document.getElementById('dpnsStats');
        const totalDomains = document.getElementById('totalDomains');
        const recentDomains = document.getElementById('recentDomains');
        const averageLength = document.getElementById('averageLength');
        const contestedDomains = document.getElementById('contestedDomains');

        totalDomains.textContent = stats.totalSample.toLocaleString();
        recentDomains.textContent = stats.recent24h;
        averageLength.textContent = `${stats.averageLength} chars`;
        contestedDomains.textContent = stats.contested;

        dpnsStats.style.display = 'block';
    }

    calculateDomainStats(allDomains, recentDomains) {
        const stats = {
            totalSample: allDomains?.length || 0,
            recent24h: recentDomains?.length || 0,
            averageLength: 0,
            contested: 0
        };

        if (allDomains && allDomains.length > 0) {
            const lengths = allDomains.map(d => (d.data?.label || d.label || '').length);
            stats.averageLength = Math.round(lengths.reduce((a, b) => a + b, 0) / lengths.length);
        }

        return stats;
    }

    // Real-time validation display
    showRealtimeValidation(username, targetElementId) {
        if (!username) {
            const targetElement = document.getElementById(targetElementId);
            if (targetElement) {
                targetElement.style.display = 'none';
            }
            return;
        }

        const validation = this.validator.validateAsYouType(username);
        const targetElement = document.getElementById(targetElementId);
        
        if (targetElement) {
            if (validation.messages.length > 0 || validation.suggestions.length > 0) {
                const messages = [...validation.messages, ...validation.suggestions];
                targetElement.innerHTML = `
                    <div class="validation-feedback ${validation.valid ? 'success' : 'error'}">
                        ${messages.map(msg => `<div>• ${msg}</div>`).join('')}
                    </div>
                `;
                targetElement.style.display = 'block';
            } else if (validation.valid) {
                targetElement.innerHTML = `
                    <div class="validation-feedback success">
                        <div>✅ Username looks good!</div>
                    </div>
                `;
                targetElement.style.display = 'block';
            } else {
                targetElement.style.display = 'none';
            }
        }
    }

    updateRegistrationFactors(username) {
        const usernameLength = document.getElementById('usernameLength');
        const baseCost = document.getElementById('baseCost');
        const lengthMultiplier = document.getElementById('lengthMultiplier');
        const totalCost = document.getElementById('totalCost');

        if (!username) {
            usernameLength.textContent = '0 characters';
            baseCost.textContent = '- credits';
            lengthMultiplier.textContent = '1x';
            totalCost.textContent = '- credits';
            return;
        }

        const costEstimate = this.validator.estimateRegistrationCost(username);
        
        usernameLength.textContent = `${username.length} characters`;
        
        if (costEstimate.valid) {
            baseCost.textContent = `${costEstimate.baseCost.toLocaleString()} credits`;
            lengthMultiplier.textContent = `${costEstimate.lengthMultiplier.toFixed(1)}x`;
            totalCost.textContent = `${costEstimate.cost.toLocaleString()} credits`;
        } else {
            baseCost.textContent = '- credits';
            lengthMultiplier.textContent = '- ';
            totalCost.textContent = '- credits';
        }
    }

    showDomainDetails(domain) {
        const modal = document.getElementById('domainModal');
        const domainDetails = document.getElementById('domainDetails');

        domainDetails.textContent = JSON.stringify(domain, null, 2);
        modal.style.display = 'flex';

        // Setup modal actions
        const resolveFromModalBtn = document.getElementById('resolveFromModalBtn');
        const exportDomainBtn = document.getElementById('exportDomainBtn');

        resolveFromModalBtn.onclick = () => {
            document.getElementById('usernameInput').value = domain.data?.label || domain.label || '';
            this.closeDomainModal();
            this.resolveUsername();
        };

        exportDomainBtn.onclick = () => this.exportDomainData(domain);
    }

    // Helper Methods
    onDomainFilterChange() {
        const filter = document.getElementById('domainFilter').value;
        const ownerFilterGroup = document.getElementById('ownerFilterGroup');
        
        ownerFilterGroup.style.display = filter === 'owner' ? 'block' : 'none';
    }

    reverseResolveFromResults() {
        if (!this.lastResolution) {
            this.showError('No resolution data to reverse');
            return;
        }

        // Use the resolved identity for reverse resolution
        document.getElementById('identityResolveInput').value = this.lastResolution.result;
        this.reverseResolve();
    }

    exportDomainData(domain) {
        const exportData = {
            domain: domain,
            network: this.currentNetwork,
            contractId: this.dpnsContractId,
            exportedAt: new Date().toISOString()
        };

        this.downloadFile(
            JSON.stringify(exportData, null, 2),
            `domain_${domain.data?.label || domain.label || Date.now()}.json`,
            'application/json'
        );

        this.logOperation('Export', 'Domain data exported', 'success');
    }

    downloadFile(content, filename, mimeType) {
        const blob = new Blob([content], { type: mimeType });
        const url = URL.createObjectURL(blob);
        
        const a = document.createElement('a');
        a.href = url;
        a.download = filename;
        document.body.appendChild(a);
        a.click();
        document.body.removeChild(a);
        URL.revokeObjectURL(url);
    }

    // Utility Methods
    isValidIdentityId(id) {
        return /^[1-9A-HJ-NP-Za-km-z]{44,}$/.test(id);
    }

    truncateId(id) {
        if (!id || id.length <= 16) return id;
        return `${id.substring(0, 8)}...${id.substring(id.length - 8)}`;
    }

    formatDate(timestamp) {
        if (!timestamp) return 'N/A';
        return new Date(timestamp).toLocaleDateString();
    }

    // UI Helper Methods
    updateStatus(status, text) {
        const statusIndicator = document.getElementById('statusIndicator');
        const statusText = document.getElementById('statusText');
        const statusDot = statusIndicator.querySelector('.status-dot');

        statusText.textContent = text;

        // Remove existing status classes
        statusDot.classList.remove('connected', 'connecting', 'error');
        
        // Add new status class
        statusDot.classList.add(status === 'connected' ? 'connected' : 
                                 status === 'connecting' ? 'connecting' : 'error');
    }

    showLoadingOverlay(message) {
        const overlay = document.getElementById('loadingOverlay');
        const loadingText = document.getElementById('loadingText');
        loadingText.textContent = message;
        overlay.style.display = 'flex';
    }

    hideLoadingOverlay() {
        const overlay = document.getElementById('loadingOverlay');
        overlay.style.display = 'none';
    }

    showError(message) {
        console.error('Error:', message);
        alert(message); // Simple error display - could be enhanced with modal
    }

    logOperation(category, message, type = 'info') {
        console.log(`[${category}] ${message}`);
        
        const dpnsLog = document.getElementById('dpnsLog');
        const timestamp = new Date().toLocaleTimeString();
        
        const logEntry = document.createElement('div');
        logEntry.className = `log-entry ${type}`;
        
        const timestampSpan = document.createElement('span');
        timestampSpan.className = 'timestamp';
        timestampSpan.textContent = `[${timestamp}] [${category}]`;
        
        const messageSpan = document.createElement('span');
        messageSpan.className = 'message';
        messageSpan.textContent = message;
        
        logEntry.appendChild(timestampSpan);
        logEntry.appendChild(messageSpan);
        
        dpnsLog.appendChild(logEntry);
        dpnsLog.scrollTop = dpnsLog.scrollHeight;
    }
}

// Global functions for UI interactions
window.loadSampleUsername = function() {
    const input = document.getElementById('usernameInput');
    input.value = 'alice';
};

window.clearUsername = function() {
    const input = document.getElementById('usernameInput');
    input.value = '';
    
    const validationInfo = document.getElementById('validationInfo');
    validationInfo.style.display = 'none';
};

window.loadSampleIdentityForReverse = function() {
    const input = document.getElementById('identityResolveInput');
    input.value = '5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk';
};

window.clearReverseInput = function() {
    const input = document.getElementById('identityResolveInput');
    input.value = '';
};

window.closeDomainModal = function() {
    const modal = document.getElementById('domainModal');
    modal.style.display = 'none';
};

window.closeValidationModal = function() {
    const modal = document.getElementById('validationModal');
    modal.style.display = 'none';
};

window.proceedToResolve = function() {
    const username = document.getElementById('validateUsernameInput').value;
    document.getElementById('usernameInput').value = username;
    window.closeValidationModal();
    
    const app = window.dpnsResolverApp;
    if (app) {
        app.resolveUsername();
    }
};

window.clearDpnsLog = function() {
    const dpnsLog = document.getElementById('dpnsLog');
    dpnsLog.innerHTML = '<div class="log-entry"><span class="timestamp">[System]</span><span class="message">Log cleared</span></div>';
};

window.exportDpnsLog = function() {
    const dpnsLog = document.getElementById('dpnsLog');
    const logEntries = Array.from(dpnsLog.querySelectorAll('.log-entry')).map(entry => {
        const timestamp = entry.querySelector('.timestamp').textContent;
        const message = entry.querySelector('.message').textContent;
        return `${timestamp} ${message}`;
    });

    const logData = logEntries.join('\n');
    const dataBlob = new Blob([logData], { type: 'text/plain' });
    const url = URL.createObjectURL(dataBlob);

    const a = document.createElement('a');
    a.href = url;
    a.download = `dpns_resolver_log_${Date.now()}.txt`;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
};

window.toggleLogLevel = function() {
    const app = window.dpnsResolverApp;
    if (app) {
        app.debugMode = !app.debugMode;
        const btn = event.target;
        btn.textContent = app.debugMode ? 'Debug ON' : 'Toggle Debug';
        app.logOperation('Debug', `Debug mode ${app.debugMode ? 'enabled' : 'disabled'}`, 'debug');
    }
};

// Initialize application when DOM is ready
document.addEventListener('DOMContentLoaded', () => {
    const app = new DPNSResolver();
    window.dpnsResolverApp = app; // Make available globally for UI functions
    app.init();
});