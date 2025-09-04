/**
 * Document Explorer - Dash Platform WASM SDK Sample Application
 * Demonstrates advanced document querying with filtering and sorting
 */

import init, { WasmSdk, WasmSdkBuilder } from '../../pkg/wasm_sdk.js';
import { KNOWN_CONTRACTS, getContractSchema, getDocumentFields, getQueryableFields, getSampleQueries, formatFieldValue } from './contract-schemas.js';

class DocumentExplorer {
    constructor() {
        this.sdk = null;
        this.currentNetwork = 'testnet';
        this.isInitialized = false;
        this.currentContract = null;
        this.currentDocumentType = null;
        this.queryHistory = [];
        this.lastQuery = null;
        
        // Bind methods
        this.initializeSDK = this.initializeSDK.bind(this);
        this.loadContract = this.loadContract.bind(this);
        this.executeQuery = this.executeQuery.bind(this);
    }

    async init() {
        try {
            await this.initializeEventListeners();
            await this.initializeSDK();
            this.setupContractButtons();
            this.logQuery('System', 'Document Explorer initialized successfully');
        } catch (error) {
            this.showError('Failed to initialize application: ' + error.message);
            this.logQuery('System', `Initialization failed: ${error.message}`, 'error');
        }
    }

    initializeEventListeners() {
        // Network selection
        const networkSelect = document.getElementById('networkSelect');
        networkSelect.addEventListener('change', async (e) => {
            this.currentNetwork = e.target.value;
            await this.initializeSDK();
            this.setupContractButtons(); // Refresh contract IDs for new network
        });

        // Custom contract loading
        const loadContractBtn = document.getElementById('loadContractBtn');
        loadContractBtn.addEventListener('click', this.loadCustomContract.bind(this));

        // Document type selection
        const documentTypeSelect = document.getElementById('documentTypeSelect');
        documentTypeSelect.addEventListener('change', this.onDocumentTypeChange.bind(this));

        // Query execution
        const executeQueryBtn = document.getElementById('executeQueryBtn');
        executeQueryBtn.addEventListener('click', this.executeQuery);

        // Query management
        const clearQueryBtn = document.getElementById('clearQueryBtn');
        clearQueryBtn.addEventListener('click', this.clearQuery.bind(this));

        const exportQueryBtn = document.getElementById('exportQueryBtn');
        exportQueryBtn.addEventListener('click', this.exportQuery.bind(this));

        // Condition builders
        const addConditionBtn = document.getElementById('addConditionBtn');
        addConditionBtn.addEventListener('click', this.addWhereCondition.bind(this));

        const addOrderByBtn = document.getElementById('addOrderByBtn');
        addOrderByBtn.addEventListener('click', this.addOrderByCondition.bind(this));

        // Results export
        const exportResultsBtn = document.getElementById('exportResultsBtn');
        exportResultsBtn.addEventListener('click', this.exportResults.bind(this));

        const exportCsvBtn = document.getElementById('exportCsvBtn');
        exportCsvBtn.addEventListener('click', this.exportResultsCSV.bind(this));

        // History management
        const clearHistoryBtn = document.getElementById('clearHistoryBtn');
        clearHistoryBtn.addEventListener('click', this.clearHistory.bind(this));

        const exportHistoryBtn = document.getElementById('exportHistoryBtn');
        exportHistoryBtn.addEventListener('click', this.exportHistory.bind(this));
    }

    async initializeSDK() {
        try {
            this.updateStatus('connecting', 'Initializing SDK...');
            this.logQuery('SDK', `Initializing for ${this.currentNetwork} network`);

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
            this.logQuery('SDK', `Successfully connected to ${this.currentNetwork}`, 'success');

        } catch (error) {
            this.updateStatus('error', 'Connection failed');
            this.logQuery('SDK', `Connection failed: ${error.message}`, 'error');
            throw error;
        }
    }

    setupContractButtons() {
        const contractButtons = document.querySelectorAll('.contract-btn');
        contractButtons.forEach(btn => {
            btn.addEventListener('click', (e) => {
                // Remove selection from other buttons
                contractButtons.forEach(b => b.classList.remove('selected'));
                
                // Select current button
                btn.classList.add('selected');
                
                // Load the selected contract
                const contractName = btn.dataset.contract;
                this.loadKnownContract(contractName);
            });
        });
    }

    async loadKnownContract(contractName) {
        const schema = getContractSchema(contractName, this.currentNetwork);
        if (!schema) {
            this.showError('Unknown contract: ' + contractName);
            return;
        }

        try {
            this.showLoading('Loading contract...');
            this.logQuery('Contract', `Loading ${schema.name} contract`);

            // Try to fetch actual contract from network
            const contractData = await this.sdk.get_data_contract(schema.contractId);
            
            this.currentContract = {
                id: schema.contractId,
                name: schema.name,
                description: schema.description,
                schema: schema,
                data: contractData
            };

            this.displayContractInfo();
            this.populateDocumentTypes(schema);
            this.showQueryForm();
            
            this.logQuery('Contract', `${schema.name} contract loaded successfully`, 'success');

        } catch (error) {
            this.showError('Failed to load contract: ' + error.message);
            this.logQuery('Contract', `Failed to load ${contractName}: ${error.message}`, 'error');
        } finally {
            this.hideLoading();
        }
    }

    async loadCustomContract() {
        const contractId = document.getElementById('customContractId').value.trim();
        
        if (!contractId) {
            this.showError('Please enter a contract ID');
            return;
        }

        if (!this.isValidContractId(contractId)) {
            this.showError('Invalid contract ID format');
            return;
        }

        try {
            this.showLoading('Loading custom contract...');
            this.logQuery('Contract', `Loading custom contract: ${contractId}`);

            const contractData = await this.sdk.get_data_contract(contractId);
            
            if (contractData) {
                this.currentContract = {
                    id: contractId,
                    name: 'Custom Contract',
                    description: 'User-specified data contract',
                    data: contractData,
                    schema: this.inferSchemaFromContract(contractData)
                };

                this.displayContractInfo();
                this.populateCustomDocumentTypes(contractData);
                this.showQueryForm();
                
                this.logQuery('Contract', 'Custom contract loaded successfully', 'success');
            } else {
                this.showError('Contract not found or inaccessible');
                this.logQuery('Contract', 'Custom contract not found', 'error');
            }

        } catch (error) {
            this.showError('Failed to load custom contract: ' + error.message);
            this.logQuery('Contract', `Failed to load custom contract: ${error.message}`, 'error');
        } finally {
            this.hideLoading();
        }
    }

    displayContractInfo() {
        const contractInfo = document.getElementById('contractInfo');
        const contractDetails = document.getElementById('contractDetails');

        const contractJson = {
            id: this.currentContract.id,
            name: this.currentContract.name,
            description: this.currentContract.description,
            version: this.currentContract.data?.version || 'Unknown',
            ownerId: this.currentContract.data?.ownerId || 'Unknown',
            documentTypes: Object.keys(this.currentContract.schema.documents || {})
        };

        contractDetails.textContent = JSON.stringify(contractJson, null, 2);
        contractInfo.style.display = 'block';
    }

    populateDocumentTypes(schema) {
        const documentTypeSelect = document.getElementById('documentTypeSelect');
        const documentTypesDiv = document.getElementById('documentTypes');

        // Clear existing options
        documentTypeSelect.innerHTML = '<option value="">Select document type...</option>';
        documentTypesDiv.innerHTML = '';

        // Add document types
        Object.keys(schema.documents).forEach(docType => {
            // Add to select dropdown
            const option = document.createElement('option');
            option.value = docType;
            option.textContent = docType;
            documentTypeSelect.appendChild(option);

            // Add as button
            const btn = document.createElement('button');
            btn.className = 'document-type-btn';
            btn.textContent = docType;
            btn.addEventListener('click', () => {
                documentTypeSelect.value = docType;
                this.onDocumentTypeChange();
            });
            documentTypesDiv.appendChild(btn);
        });
    }

    populateCustomDocumentTypes(contractData) {
        const documentTypeSelect = document.getElementById('documentTypeSelect');
        const documentTypesDiv = document.getElementById('documentTypes');

        // Clear existing options
        documentTypeSelect.innerHTML = '<option value="">Select document type...</option>';
        documentTypesDiv.innerHTML = '';

        // Extract document types from contract data
        const documentTypes = contractData.documents ? Object.keys(contractData.documents) : [];
        
        documentTypes.forEach(docType => {
            // Add to select dropdown
            const option = document.createElement('option');
            option.value = docType;
            option.textContent = docType;
            documentTypeSelect.appendChild(option);

            // Add as button
            const btn = document.createElement('button');
            btn.className = 'document-type-btn';
            btn.textContent = docType;
            btn.addEventListener('click', () => {
                documentTypeSelect.value = docType;
                this.onDocumentTypeChange();
            });
            documentTypesDiv.appendChild(btn);
        });
    }

    onDocumentTypeChange() {
        const documentType = document.getElementById('documentTypeSelect').value;
        
        if (documentType) {
            this.currentDocumentType = documentType;
            this.updateQueryBuilderFields();
            this.loadSampleQueries();
        }
    }

    updateQueryBuilderFields() {
        const contractName = Object.keys(KNOWN_CONTRACTS).find(name => 
            KNOWN_CONTRACTS[name].id[this.currentNetwork] === this.currentContract.id
        ) || 'custom';

        const fields = contractName !== 'custom' 
            ? getQueryableFields(contractName, this.currentDocumentType)
            : this.extractFieldsFromContract();

        // Update WHERE clause field selects
        const whereSelects = document.querySelectorAll('#whereConditions .field-select');
        whereSelects.forEach(select => {
            this.populateFieldSelect(select, fields);
        });

        // Update ORDER BY clause field selects  
        const orderBySelects = document.querySelectorAll('#orderByConditions .field-select');
        orderBySelects.forEach(select => {
            this.populateFieldSelect(select, fields);
        });
    }

    populateFieldSelect(select, fields) {
        select.innerHTML = '<option value="">Select field...</option>';
        
        fields.forEach(field => {
            const option = document.createElement('option');
            option.value = field.name;
            option.textContent = `${field.name} (${field.type})`;
            option.title = field.description || '';
            select.appendChild(option);
        });
    }

    loadSampleQueries() {
        const contractName = Object.keys(KNOWN_CONTRACTS).find(name => 
            KNOWN_CONTRACTS[name].id[this.currentNetwork] === this.currentContract.id
        );

        if (!contractName) return;

        const samples = getSampleQueries(contractName, this.currentDocumentType, this.currentNetwork);
        
        if (samples.length > 0) {
            // Add sample query buttons to the interface
            this.addSampleQueryButtons(samples);
        }
    }

    addSampleQueryButtons(samples) {
        // Find or create sample queries section
        let samplesDiv = document.getElementById('sampleQueries');
        if (!samplesDiv) {
            samplesDiv = document.createElement('div');
            samplesDiv.id = 'sampleQueries';
            samplesDiv.innerHTML = '<h4>📚 Sample Queries</h4><div class="sample-buttons"></div>';
            document.getElementById('queryForm').appendChild(samplesDiv);
        }

        const buttonsDiv = samplesDiv.querySelector('.sample-buttons');
        buttonsDiv.innerHTML = '';

        samples.forEach(sample => {
            const btn = document.createElement('button');
            btn.className = 'btn btn-secondary btn-small';
            btn.textContent = sample.name;
            btn.addEventListener('click', () => {
                this.applySampleQuery(sample);
            });
            buttonsDiv.appendChild(btn);
        });
    }

    applySampleQuery(sample) {
        // Clear existing conditions
        this.clearQuery();

        // Apply WHERE conditions
        sample.where.forEach(condition => {
            this.addWhereCondition();
            const lastRow = document.querySelector('#whereConditions .condition-row:last-child');
            lastRow.querySelector('.field-select').value = condition[0];
            lastRow.querySelector('.operator-select').value = condition[1];
            lastRow.querySelector('.value-input').value = condition[2];
        });

        // Apply ORDER BY conditions
        sample.orderBy.forEach(orderBy => {
            this.addOrderByCondition();
            const lastRow = document.querySelector('#orderByConditions .orderby-row:last-child');
            lastRow.querySelector('.field-select').value = orderBy[0];
            lastRow.querySelector('.direction-select').value = orderBy[1];
        });

        // Apply limit
        document.getElementById('limitInput').value = sample.limit || 10;

        this.logQuery('Sample', `Applied sample query: ${sample.name}`);
    }

    addWhereCondition() {
        const whereConditions = document.getElementById('whereConditions');
        const newRow = document.createElement('div');
        newRow.className = 'condition-row';
        newRow.innerHTML = `
            <select class="field-select">
                <option value="">Select field...</option>
            </select>
            <select class="operator-select">
                <option value="=">=</option>
                <option value="!="≠</option>
                <option value=">">&gt;</option>
                <option value=">=">&ge;</option>
                <option value="<">&lt;</option>
                <option value="<=">&le;</option>
                <option value="in">in</option>
                <option value="startsWith">starts with</option>
            </select>
            <input type="text" class="value-input" placeholder="Enter value...">
            <button type="button" class="btn btn-remove" onclick="removeCondition(this)">×</button>
        `;
        
        whereConditions.appendChild(newRow);
        
        // Populate fields if we have a current document type
        if (this.currentDocumentType) {
            this.updateQueryBuilderFields();
        }
    }

    addOrderByCondition() {
        const orderByConditions = document.getElementById('orderByConditions');
        const newRow = document.createElement('div');
        newRow.className = 'orderby-row';
        newRow.innerHTML = `
            <select class="field-select">
                <option value="">Select field...</option>
            </select>
            <select class="direction-select">
                <option value="asc">Ascending</option>
                <option value="desc">Descending</option>
            </select>
            <button type="button" class="btn btn-remove" onclick="removeOrderBy(this)">×</button>
        `;
        
        orderByConditions.appendChild(newRow);
        
        // Populate fields if we have a current document type
        if (this.currentDocumentType) {
            this.updateQueryBuilderFields();
        }
    }

    async executeQuery() {
        if (!this.isInitialized) {
            this.showError('SDK not initialized. Please wait for connection.');
            return;
        }

        if (!this.currentContract || !this.currentDocumentType) {
            this.showError('Please select a contract and document type first.');
            return;
        }

        try {
            const queryParams = this.buildQueryParameters();
            
            this.showResultsLoading();
            this.hideResultsError();
            
            const startTime = Date.now();
            this.logQuery('Query', `Executing: ${this.currentDocumentType} on ${this.currentContract.name}`);

            // Execute the query
            const results = await this.sdk.get_documents(
                this.currentContract.id,
                this.currentDocumentType,
                queryParams.where,
                queryParams.orderBy,
                queryParams.limit,
                queryParams.offset
            );

            const queryTime = Date.now() - startTime;
            this.displayResults(results, queryTime);
            this.addToHistory(queryParams, results, queryTime);
            
            this.logQuery('Query', `Query completed in ${queryTime}ms, found ${results?.length || 0} documents`, 'success');

        } catch (error) {
            this.showResultsError('Query failed: ' + error.message);
            this.logQuery('Query', `Query failed: ${error.message}`, 'error');
        } finally {
            this.hideResultsLoading();
        }
    }

    buildQueryParameters() {
        const limit = parseInt(document.getElementById('limitInput').value) || 10;
        const offset = parseInt(document.getElementById('offsetInput').value) || 0;

        // Build WHERE clause
        const whereConditions = [];
        const whereRows = document.querySelectorAll('#whereConditions .condition-row');
        
        whereRows.forEach(row => {
            const field = row.querySelector('.field-select').value;
            const operator = row.querySelector('.operator-select').value;
            const value = row.querySelector('.value-input').value;
            
            if (field && operator && value) {
                whereConditions.push([field, operator, this.parseValue(value)]);
            }
        });

        // Build ORDER BY clause
        const orderByConditions = [];
        const orderByRows = document.querySelectorAll('#orderByConditions .orderby-row');
        
        orderByRows.forEach(row => {
            const field = row.querySelector('.field-select').value;
            const direction = row.querySelector('.direction-select').value;
            
            if (field && direction) {
                orderByConditions.push([field, direction]);
            }
        });

        const params = {
            where: whereConditions.length > 0 ? JSON.stringify(whereConditions) : null,
            orderBy: orderByConditions.length > 0 ? JSON.stringify(orderByConditions) : null,
            limit,
            offset
        };

        // Display generated query
        this.displayGeneratedQuery(params);
        
        return params;
    }

    parseValue(value) {
        // Try to parse as number
        if (!isNaN(value) && !isNaN(parseFloat(value))) {
            return parseFloat(value);
        }
        
        // Try to parse as boolean
        if (value.toLowerCase() === 'true') return true;
        if (value.toLowerCase() === 'false') return false;
        
        // Try to parse as JSON array for 'in' operations
        if (value.startsWith('[') && value.endsWith(']')) {
            try {
                return JSON.parse(value);
            } catch (e) {
                // Fall back to string
            }
        }
        
        // Return as string
        return value;
    }

    displayGeneratedQuery(params) {
        const generatedQuery = document.getElementById('generatedQuery');
        const queryCode = document.getElementById('queryCode');

        const queryExample = `
// Generated Query Code
const results = await sdk.get_documents(
    '${this.currentContract.id}',  // Contract ID
    '${this.currentDocumentType}',              // Document Type
    ${params.where || 'null'},                 // WHERE clause
    ${params.orderBy || 'null'},               // ORDER BY clause
    ${params.limit},                           // Limit
    ${params.offset}                           // Offset
);

console.log(\`Found \${results.length} documents\`);`;

        queryCode.textContent = queryExample;
        generatedQuery.style.display = 'block';
    }

    displayResults(results, queryTime) {
        const resultsHeader = document.getElementById('resultsHeader');
        const resultsContainer = document.getElementById('resultsContainer');
        const resultsCount = document.getElementById('resultsCount');
        const queryTimeSpan = document.getElementById('queryTime');

        // Update header
        resultsCount.textContent = `${results?.length || 0} documents found`;
        queryTimeSpan.textContent = `Query time: ${queryTime}ms`;
        resultsHeader.style.display = 'flex';

        // Clear previous results
        resultsContainer.innerHTML = '';

        if (!results || results.length === 0) {
            resultsContainer.innerHTML = `
                <div class="empty-state">
                    <div class="empty-icon">📭</div>
                    <h4>No Documents Found</h4>
                    <p>Try adjusting your query parameters or removing some filters</p>
                </div>
            `;
            return;
        }

        // Create document grid
        const documentGrid = document.createElement('div');
        documentGrid.className = 'document-grid';

        results.forEach((doc, index) => {
            const docElement = this.createDocumentElement(doc, index);
            documentGrid.appendChild(docElement);
        });

        resultsContainer.appendChild(documentGrid);
        this.lastQuery = { results, queryTime, params: this.buildQueryParameters() };
    }

    createDocumentElement(doc, index) {
        const docDiv = document.createElement('div');
        docDiv.className = 'document-item';
        docDiv.addEventListener('click', () => this.showDocumentDetails(doc));

        const header = document.createElement('div');
        header.className = 'document-header';
        header.innerHTML = `
            <span class="document-number">#${index + 1}</span>
            <span class="document-id">${doc.id || 'No ID'}</span>
        `;

        const meta = document.createElement('div');
        meta.className = 'document-meta';
        meta.innerHTML = `
            <span><strong>Owner:</strong> ${this.truncateId(doc.ownerId || 'Unknown')}</span>
            <span><strong>Created:</strong> ${this.formatDate(doc.createdAt)}</span>
        `;

        const preview = document.createElement('div');
        preview.className = 'document-preview';
        preview.textContent = JSON.stringify(doc.data || doc, null, 2).substring(0, 200) + '...';

        docDiv.appendChild(header);
        docDiv.appendChild(meta);
        docDiv.appendChild(preview);

        return docDiv;
    }

    showDocumentDetails(doc) {
        const modal = document.getElementById('documentModal');
        const documentDetails = document.getElementById('documentDetails');

        documentDetails.textContent = JSON.stringify(doc, null, 2);
        modal.style.display = 'flex';

        // Setup export button
        const exportBtn = document.getElementById('exportDocumentBtn');
        exportBtn.onclick = () => this.exportSingleDocument(doc);
    }

    showQueryForm() {
        const queryForm = document.getElementById('queryForm');
        queryForm.style.display = 'block';
    }

    clearQuery() {
        // Clear WHERE conditions (keep first row)
        const whereConditions = document.getElementById('whereConditions');
        const firstWhereRow = whereConditions.querySelector('.condition-row');
        whereConditions.innerHTML = '';
        if (firstWhereRow) {
            // Reset first row
            firstWhereRow.querySelectorAll('select, input').forEach(el => el.value = '');
            whereConditions.appendChild(firstWhereRow);
        }

        // Clear ORDER BY conditions (keep first row)
        const orderByConditions = document.getElementById('orderByConditions');
        const firstOrderRow = orderByConditions.querySelector('.orderby-row');
        orderByConditions.innerHTML = '';
        if (firstOrderRow) {
            // Reset first row
            firstOrderRow.querySelectorAll('select').forEach(el => el.value = '');
            orderByConditions.appendChild(firstOrderRow);
        }

        // Reset form values
        document.getElementById('limitInput').value = 10;
        document.getElementById('offsetInput').value = 0;
        
        // Hide generated query
        document.getElementById('generatedQuery').style.display = 'none';
        
        this.logQuery('UI', 'Query form cleared');
    }

    // Utility Methods
    isValidContractId(id) {
        return /^[1-9A-HJ-NP-Za-km-z]{44,}$/.test(id);
    }

    truncateId(id) {
        if (!id || id.length <= 16) return id;
        return `${id.substring(0, 8)}...${id.substring(id.length - 8)}`;
    }

    formatDate(timestamp) {
        if (!timestamp) return 'N/A';
        return new Date(timestamp).toLocaleString();
    }

    inferSchemaFromContract(contractData) {
        // Basic schema inference from contract data
        const documents = {};
        
        if (contractData.documents) {
            Object.keys(contractData.documents).forEach(docType => {
                documents[docType] = {
                    description: `${docType} documents`,
                    fields: this.inferFieldsFromDocumentType(contractData.documents[docType])
                };
            });
        }

        return { documents };
    }

    inferFieldsFromDocumentType(docTypeData) {
        const fields = {
            '$ownerId': { type: 'identifier', description: 'Document owner identity' },
            '$createdAt': { type: 'date', description: 'Creation timestamp' },
            '$updatedAt': { type: 'date', description: 'Last update timestamp' }
        };

        // Add fields from document properties if available
        if (docTypeData.properties) {
            Object.keys(docTypeData.properties).forEach(prop => {
                const propData = docTypeData.properties[prop];
                fields[prop] = {
                    type: propData.type || 'string',
                    description: propData.description || `${prop} field`
                };
            });
        }

        return fields;
    }

    extractFieldsFromContract() {
        if (!this.currentContract.data || !this.currentContract.data.documents) {
            return [
                { name: '$ownerId', type: 'identifier', description: 'Document owner' },
                { name: '$createdAt', type: 'date', description: 'Creation timestamp' },
                { name: '$updatedAt', type: 'date', description: 'Update timestamp' }
            ];
        }

        const docType = this.currentContract.data.documents[this.currentDocumentType];
        if (!docType || !docType.properties) {
            return [
                { name: '$ownerId', type: 'identifier', description: 'Document owner' },
                { name: '$createdAt', type: 'date', description: 'Creation timestamp' }
            ];
        }

        const fields = [];
        
        // Add system fields
        fields.push(
            { name: '$ownerId', type: 'identifier', description: 'Document owner' },
            { name: '$createdAt', type: 'date', description: 'Creation timestamp' },
            { name: '$updatedAt', type: 'date', description: 'Update timestamp' }
        );

        // Add document-specific fields
        Object.keys(docType.properties).forEach(prop => {
            const propData = docType.properties[prop];
            fields.push({
                name: prop,
                type: propData.type || 'string',
                description: propData.description || `${prop} field`
            });
        });

        return fields;
    }

    addToHistory(queryParams, results, queryTime) {
        const historyEntry = {
            timestamp: new Date().toISOString(),
            contract: this.currentContract.name,
            contractId: this.currentContract.id,
            documentType: this.currentDocumentType,
            params: queryParams,
            resultCount: results?.length || 0,
            queryTime,
            success: true
        };

        this.queryHistory.unshift(historyEntry); // Add to beginning
        
        // Keep only last 50 queries
        if (this.queryHistory.length > 50) {
            this.queryHistory = this.queryHistory.slice(0, 50);
        }

        this.updateHistoryDisplay();
    }

    updateHistoryDisplay() {
        const queryHistory = document.getElementById('queryHistory');
        queryHistory.innerHTML = '';

        this.queryHistory.forEach(entry => {
            const historyDiv = document.createElement('div');
            historyDiv.className = `history-entry ${entry.success ? 'success' : 'error'}`;
            
            const timestamp = document.createElement('span');
            timestamp.className = 'timestamp';
            timestamp.textContent = `[${new Date(entry.timestamp).toLocaleTimeString()}]`;
            
            const query = document.createElement('span');
            query.className = 'query';
            query.textContent = `${entry.contract}/${entry.documentType} → ${entry.resultCount} docs (${entry.queryTime}ms)`;
            
            historyDiv.appendChild(timestamp);
            historyDiv.appendChild(query);
            queryHistory.appendChild(historyDiv);
        });
    }

    // Export Methods
    exportResults() {
        if (!this.lastQuery || !this.lastQuery.results) {
            this.showError('No results to export');
            return;
        }

        const exportData = {
            query: {
                contract: this.currentContract.name,
                contractId: this.currentContract.id,
                documentType: this.currentDocumentType,
                parameters: this.lastQuery.params
            },
            results: this.lastQuery.results,
            metadata: {
                resultCount: this.lastQuery.results.length,
                queryTime: this.lastQuery.queryTime,
                exportedAt: new Date().toISOString(),
                network: this.currentNetwork
            }
        };

        this.downloadFile(
            JSON.stringify(exportData, null, 2),
            `documents_${this.currentDocumentType}_${Date.now()}.json`,
            'application/json'
        );

        this.logQuery('Export', `Exported ${this.lastQuery.results.length} documents as JSON`, 'success');
    }

    exportResultsCSV() {
        if (!this.lastQuery || !this.lastQuery.results) {
            this.showError('No results to export');
            return;
        }

        const results = this.lastQuery.results;
        if (results.length === 0) {
            this.showError('No data to export');
            return;
        }

        // Convert to CSV
        const csv = this.convertToCSV(results);
        
        this.downloadFile(
            csv,
            `documents_${this.currentDocumentType}_${Date.now()}.csv`,
            'text/csv'
        );

        this.logQuery('Export', `Exported ${results.length} documents as CSV`, 'success');
    }

    convertToCSV(data) {
        if (!data || data.length === 0) return '';

        // Get all unique keys from all objects
        const allKeys = new Set();
        data.forEach(item => {
            Object.keys(item).forEach(key => allKeys.add(key));
            if (item.data) {
                Object.keys(item.data).forEach(key => allKeys.add(`data.${key}`));
            }
        });

        const headers = Array.from(allKeys);
        const csvRows = [headers.join(',')];

        data.forEach(item => {
            const row = headers.map(header => {
                let value;
                if (header.startsWith('data.')) {
                    const dataKey = header.substring(5);
                    value = item.data?.[dataKey];
                } else {
                    value = item[header];
                }
                
                // Handle different value types
                if (value === null || value === undefined) {
                    return '';
                } else if (typeof value === 'object') {
                    return `"${JSON.stringify(value).replace(/"/g, '""')}"`;
                } else {
                    return `"${String(value).replace(/"/g, '""')}"`;
                }
            });
            csvRows.push(row.join(','));
        });

        return csvRows.join('\n');
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

    exportSingleDocument(doc) {
        const exportData = {
            document: doc,
            metadata: {
                contract: this.currentContract.name,
                contractId: this.currentContract.id,
                documentType: this.currentDocumentType,
                exportedAt: new Date().toISOString(),
                network: this.currentNetwork
            }
        };

        this.downloadFile(
            JSON.stringify(exportData, null, 2),
            `document_${doc.id || Date.now()}.json`,
            'application/json'
        );

        this.logQuery('Export', 'Single document exported', 'success');
    }

    exportQuery() {
        const params = this.buildQueryParameters();
        const queryCode = `
// Copy this code to use in your application
import init, { WasmSdk, WasmSdkBuilder } from '@dashevo/dash-wasm-sdk';

async function queryDocuments() {
    // Initialize SDK
    await init();
    const builder = WasmSdkBuilder.new_${this.currentNetwork}();
    const sdk = builder.build();

    // Execute query
    const results = await sdk.get_documents(
        '${this.currentContract.id}',  // Contract: ${this.currentContract.name}
        '${this.currentDocumentType}',              // Document Type
        ${params.where || 'null'},                 // WHERE clause
        ${params.orderBy || 'null'},               // ORDER BY clause
        ${params.limit},                           // Limit
        ${params.offset}                           // Offset
    );

    console.log(\`Found \${results.length} documents\`);
    return results;
}`;

        navigator.clipboard.writeText(queryCode).then(() => {
            this.logQuery('Export', 'Query code copied to clipboard', 'success');
            
            // Show temporary success message
            const btn = document.getElementById('exportQueryBtn');
            const originalText = btn.textContent;
            btn.textContent = '✅ Copied!';
            btn.style.background = '#28a745';
            
            setTimeout(() => {
                btn.textContent = originalText;
                btn.style.background = '';
            }, 2000);
        });
    }

    clearHistory() {
        this.queryHistory = [];
        this.updateHistoryDisplay();
        this.logQuery('History', 'Query history cleared');
    }

    exportHistory() {
        const historyData = {
            queryHistory: this.queryHistory,
            metadata: {
                totalQueries: this.queryHistory.length,
                exportedAt: new Date().toISOString(),
                network: this.currentNetwork
            }
        };

        this.downloadFile(
            JSON.stringify(historyData, null, 2),
            `query_history_${Date.now()}.json`,
            'application/json'
        );

        this.logQuery('Export', `Exported query history (${this.queryHistory.length} entries)`, 'success');
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

    showLoading(message) {
        console.log('Loading:', message);
        // Loading is handled by individual sections
    }

    hideLoading() {
        // Loading is handled by individual sections
    }

    showResultsLoading() {
        const loading = document.getElementById('loadingResults');
        loading.style.display = 'flex';
    }

    hideResultsLoading() {
        const loading = document.getElementById('loadingResults');
        loading.style.display = 'none';
    }

    showResultsError(message) {
        const errorDiv = document.getElementById('errorResults');
        errorDiv.textContent = message;
        errorDiv.style.display = 'block';
    }

    hideResultsError() {
        const errorDiv = document.getElementById('errorResults');
        errorDiv.style.display = 'none';
    }

    showError(message) {
        this.showResultsError(message);
        
        // Auto-hide after 10 seconds
        setTimeout(() => {
            this.hideResultsError();
        }, 10000);
    }

    logQuery(category, message, type = 'info') {
        console.log(`[${category}] ${message}`);
        
        const queryHistory = document.getElementById('queryHistory');
        const timestamp = new Date().toLocaleTimeString();
        
        const logEntry = document.createElement('div');
        logEntry.className = `history-entry ${type}`;
        
        const timestampSpan = document.createElement('span');
        timestampSpan.className = 'timestamp';
        timestampSpan.textContent = `[${timestamp}] [${category}]`;
        
        const messageSpan = document.createElement('span');
        messageSpan.className = 'query';
        messageSpan.textContent = message;
        
        logEntry.appendChild(timestampSpan);
        logEntry.appendChild(messageSpan);
        
        queryHistory.appendChild(logEntry);
        queryHistory.scrollTop = queryHistory.scrollHeight;
    }
}

// Global functions for UI interactions
window.removeCondition = function(button) {
    const row = button.closest('.condition-row');
    if (row.parentElement.children.length > 1) {
        row.remove();
    }
};

window.removeOrderBy = function(button) {
    const row = button.closest('.orderby-row');
    if (row.parentElement.children.length > 1) {
        row.remove();
    }
};

window.closeDocumentModal = function() {
    const modal = document.getElementById('documentModal');
    modal.style.display = 'none';
};

// Initialize application when DOM is ready
document.addEventListener('DOMContentLoaded', () => {
    const app = new DocumentExplorer();
    app.init();
});