/**
 * DPNS Username Validation Utilities
 * Comprehensive validation rules and helpers for Dash Platform naming
 */

export class DPNSValidator {
    constructor() {
        // DPNS validation constants
        this.MIN_LENGTH = 3;
        this.MAX_LENGTH = 63;
        this.RESERVED_WORDS = [
            'dash', 'admin', 'root', 'www', 'api', 'mail', 'ftp',
            'localhost', 'test', 'dev', 'staging', 'prod', 'production'
        ];
        this.HOMOGRAPH_CHARS = new Map([
            // Cyrillic to Latin lookalikes
            ['а', 'a'], ['е', 'e'], ['о', 'o'], ['р', 'p'], ['с', 'c'],
            ['х', 'x'], ['у', 'y'], ['В', 'B'], ['Н', 'H'], ['К', 'K'],
            ['М', 'M'], ['Р', 'P'], ['С', 'C'], ['Т', 'T'], ['Х', 'X'],
            // Greek to Latin lookalikes
            ['α', 'a'], ['ο', 'o'], ['ρ', 'p'], ['υ', 'y'], ['Α', 'A'],
            ['Β', 'B'], ['Ε', 'E'], ['Ζ', 'Z'], ['Η', 'H'], ['Ι', 'I'],
            ['Κ', 'K'], ['Μ', 'M'], ['Ν', 'N'], ['Ο', 'O'], ['Ρ', 'P'],
            ['Τ', 'T'], ['Υ', 'Y'], ['Χ', 'X']
        ]);
    }

    /**
     * Comprehensive username validation
     */
    validateUsername(username) {
        const result = {
            username,
            valid: true,
            errors: [],
            warnings: [],
            details: {
                length: username.length,
                characterCheck: true,
                formatCheck: true,
                homographCheck: true,
                reservedCheck: true
            }
        };

        // Length validation
        if (username.length < this.MIN_LENGTH) {
            result.valid = false;
            result.errors.push(`Username too short (minimum ${this.MIN_LENGTH} characters)`);
            result.details.length = `${username.length} (too short)`;
        } else if (username.length > this.MAX_LENGTH) {
            result.valid = false;
            result.errors.push(`Username too long (maximum ${this.MAX_LENGTH} characters)`);
            result.details.length = `${username.length} (too long)`;
        }

        // Character validation (basic ASCII + common international)
        const allowedPattern = /^[a-zA-Z0-9._-]+$/;
        if (!allowedPattern.test(username)) {
            result.valid = false;
            result.errors.push('Username contains invalid characters');
            result.details.characterCheck = false;
            
            const invalidChars = username.split('').filter(char => !allowedPattern.test(char));
            result.warnings.push(`Invalid characters found: ${[...new Set(invalidChars)].join(', ')}`);
        }

        // Format validation
        if (username.startsWith('-') || username.endsWith('-')) {
            result.valid = false;
            result.errors.push('Username cannot start or end with hyphen');
            result.details.formatCheck = false;
        }

        if (username.startsWith('.') || username.endsWith('.')) {
            result.valid = false;
            result.errors.push('Username cannot start or end with period');
            result.details.formatCheck = false;
        }

        if (username.includes('..') || username.includes('--')) {
            result.valid = false;
            result.errors.push('Username cannot contain consecutive dots or hyphens');
            result.details.formatCheck = false;
        }

        // Reserved words check
        const lowerUsername = username.toLowerCase();
        if (this.RESERVED_WORDS.includes(lowerUsername)) {
            result.valid = false;
            result.errors.push('Username is reserved');
            result.details.reservedCheck = false;
        }

        // Homograph attack protection
        const homographIssues = this.checkHomographs(username);
        if (homographIssues.length > 0) {
            result.warnings.push('Potential homograph characters detected');
            result.details.homographCheck = false;
            result.details.homographIssues = homographIssues;
        }

        // Additional quality checks
        if (this.isNumericOnly(username)) {
            result.warnings.push('Username is numeric-only (may be confusing)');
        }

        if (this.hasMixedScripts(username)) {
            result.warnings.push('Username contains mixed scripts (may be confusing)');
        }

        return result;
    }

    /**
     * Check for homograph attack characters
     */
    checkHomographs(username) {
        const issues = [];
        
        for (const char of username) {
            if (this.HOMOGRAPH_CHARS.has(char)) {
                const lookalike = this.HOMOGRAPH_CHARS.get(char);
                issues.push({
                    character: char,
                    lookalike: lookalike,
                    position: username.indexOf(char)
                });
            }
        }

        return issues;
    }

    /**
     * Convert username to homograph-safe version
     */
    makeHomographSafe(username) {
        let safe = username;
        
        for (const [homograph, safe_char] of this.HOMOGRAPH_CHARS) {
            safe = safe.replaceAll(homograph, safe_char);
        }
        
        return safe;
    }

    /**
     * Check if username is numeric only
     */
    isNumericOnly(username) {
        return /^\d+$/.test(username);
    }

    /**
     * Check for mixed scripts (potential confusion)
     */
    hasMixedScripts(username) {
        const latin = /[a-zA-Z]/;
        const cyrillic = /[\u0400-\u04FF]/;
        const greek = /[\u0370-\u03FF]/;
        const arabic = /[\u0600-\u06FF]/;
        
        const scripts = [
            { name: 'latin', regex: latin },
            { name: 'cyrillic', regex: cyrillic },
            { name: 'greek', regex: greek },
            { name: 'arabic', regex: arabic }
        ];
        
        const foundScripts = scripts.filter(script => script.regex.test(username));
        return foundScripts.length > 1;
    }

    /**
     * Estimate registration cost based on username properties
     */
    estimateRegistrationCost(username) {
        const validation = this.validateUsername(username);
        
        if (!validation.valid) {
            return {
                valid: false,
                cost: 0,
                errors: validation.errors
            };
        }

        // Cost calculation (simplified - actual costs may vary)
        let baseCost = 1000000; // 0.01 DASH base cost
        
        // Length factor
        if (username.length <= 4) {
            baseCost *= 10; // Premium for short names
        } else if (username.length <= 6) {
            baseCost *= 3;  // Higher cost for short names
        } else if (username.length <= 10) {
            baseCost *= 1.5; // Slight premium
        }
        // Names > 10 characters use base cost

        // Quality factors
        if (this.isNumericOnly(username)) {
            baseCost *= 0.8; // Slight discount for numeric
        }

        if (validation.warnings.length === 0) {
            baseCost *= 1.1; // Premium for high-quality names
        }

        return {
            valid: true,
            cost: Math.floor(baseCost),
            baseCost: 1000000,
            lengthMultiplier: baseCost / 1000000,
            factors: {
                length: username.length,
                quality: validation.warnings.length === 0 ? 'high' : 'medium',
                numeric: this.isNumericOnly(username),
                homographs: validation.details.homographIssues?.length || 0
            }
        };
    }

    /**
     * Generate username suggestions based on input
     */
    generateSuggestions(baseUsername, count = 5) {
        const suggestions = [];
        const base = baseUsername.toLowerCase().replace(/[^a-z0-9]/g, '');
        
        if (base.length < this.MIN_LENGTH) {
            return ['Username too short for suggestions'];
        }

        // Basic variations
        suggestions.push(`${base}123`);
        suggestions.push(`${base}2024`);
        suggestions.push(`my${base}`);
        suggestions.push(`${base}dash`);
        suggestions.push(`${base}user`);

        // Random number variations
        for (let i = suggestions.length; i < count; i++) {
            const randomNum = Math.floor(Math.random() * 9999) + 1;
            suggestions.push(`${base}${randomNum}`);
        }

        return suggestions.slice(0, count);
    }

    /**
     * Username quality score (0-100)
     */
    calculateQualityScore(username) {
        let score = 100;
        const validation = this.validateUsername(username);

        if (!validation.valid) {
            return 0;
        }

        // Length scoring
        if (username.length <= 4) {
            score += 20; // Premium short names
        } else if (username.length <= 6) {
            score += 10;
        } else if (username.length > 20) {
            score -= 10; // Long names are less valuable
        }

        // Character quality
        if (this.isNumericOnly(username)) {
            score -= 15;
        }

        if (validation.warnings.length > 0) {
            score -= validation.warnings.length * 10;
        }

        if (validation.details.homographIssues?.length > 0) {
            score -= validation.details.homographIssues.length * 20;
        }

        // Pronounceability (basic check)
        if (this.isPronounceableEnglish(username)) {
            score += 15;
        }

        // Common word bonus
        if (this.isCommonWord(username)) {
            score += 25;
        }

        return Math.max(0, Math.min(100, score));
    }

    /**
     * Basic pronounceability check for English
     */
    isPronounceableEnglish(username) {
        // Simple heuristic: has vowels and consonants
        const vowels = /[aeiou]/i;
        const consonants = /[bcdfghjklmnpqrstvwxyz]/i;
        
        return vowels.test(username) && consonants.test(username);
    }

    /**
     * Check if username is a common English word
     */
    isCommonWord(username) {
        const commonWords = [
            'alice', 'bob', 'charlie', 'david', 'eve', 'frank', 'grace', 'henry',
            'alice', 'bob', 'carol', 'dan', 'emma', 'finn', 'gina', 'hugo',
            'love', 'peace', 'hope', 'joy', 'dream', 'star', 'moon', 'sun',
            'tech', 'code', 'dev', 'web', 'app', 'game', 'art', 'music'
        ];
        
        return commonWords.includes(username.toLowerCase());
    }

    /**
     * Real-time validation for input fields
     */
    validateAsYouType(username) {
        const result = {
            valid: true,
            messages: [],
            suggestions: []
        };

        if (username.length === 0) {
            return result;
        }

        if (username.length < this.MIN_LENGTH) {
            result.valid = false;
            result.messages.push(`${this.MIN_LENGTH - username.length} more characters needed`);
        }

        if (username.length > this.MAX_LENGTH) {
            result.valid = false;
            result.messages.push(`${username.length - this.MAX_LENGTH} characters too many`);
        }

        // Check for invalid characters as user types
        const invalidChars = username.split('').filter(char => !/[a-zA-Z0-9._-]/.test(char));
        if (invalidChars.length > 0) {
            result.valid = false;
            result.messages.push(`Invalid: ${[...new Set(invalidChars)].join(', ')}`);
        }

        // Format issues
        if (username.startsWith('-') || username.startsWith('.')) {
            result.valid = false;
            result.messages.push('Cannot start with . or -');
        }

        if (username.endsWith('-') || username.endsWith('.')) {
            result.valid = false;
            result.messages.push('Cannot end with . or -');
        }

        // Reserved words
        if (this.RESERVED_WORDS.includes(username.toLowerCase())) {
            result.valid = false;
            result.messages.push('Username is reserved');
        }

        // Quality suggestions
        if (result.valid && username.length >= this.MIN_LENGTH) {
            const score = this.calculateQualityScore(username);
            if (score < 50) {
                result.suggestions.push('Consider a more unique username for better quality score');
            }
            
            const homographs = this.checkHomographs(username);
            if (homographs.length > 0) {
                result.suggestions.push('Contains characters that may be confused with others');
            }
        }

        return result;
    }

    /**
     * Format validation results for display
     */
    formatValidationResults(validation) {
        const formatted = {
            summary: validation.valid ? 'Valid Username' : 'Invalid Username',
            status: validation.valid ? 'success' : 'error',
            details: []
        };

        // Add length info
        formatted.details.push({
            label: 'Length',
            value: `${validation.details.length} characters`,
            status: validation.details.length >= this.MIN_LENGTH && validation.details.length <= this.MAX_LENGTH ? 'good' : 'bad'
        });

        // Add character validation
        formatted.details.push({
            label: 'Characters',
            value: validation.details.characterCheck ? 'Valid' : 'Invalid',
            status: validation.details.characterCheck ? 'good' : 'bad'
        });

        // Add format validation
        formatted.details.push({
            label: 'Format',
            value: validation.details.formatCheck ? 'Valid' : 'Invalid',
            status: validation.details.formatCheck ? 'good' : 'bad'
        });

        // Add reserved word check
        formatted.details.push({
            label: 'Reserved',
            value: validation.details.reservedCheck ? 'Not Reserved' : 'Reserved Word',
            status: validation.details.reservedCheck ? 'good' : 'bad'
        });

        // Add homograph check
        formatted.details.push({
            label: 'Homograph Safe',
            value: validation.details.homographCheck ? 'Safe' : 'Contains Lookalikes',
            status: validation.details.homographCheck ? 'good' : 'warning'
        });

        // Add quality score
        const qualityScore = this.calculateQualityScore(validation.username);
        formatted.details.push({
            label: 'Quality Score',
            value: `${qualityScore}/100`,
            status: qualityScore >= 70 ? 'good' : qualityScore >= 40 ? 'warning' : 'bad'
        });

        return formatted;
    }

    /**
     * Get validation rules for display
     */
    getValidationRules() {
        return {
            length: {
                title: 'Length Requirements',
                rules: [
                    `Minimum ${this.MIN_LENGTH} characters`,
                    `Maximum ${this.MAX_LENGTH} characters`,
                    'Shorter names cost more to register'
                ]
            },
            characters: {
                title: 'Allowed Characters',
                rules: [
                    'Letters: a-z, A-Z',
                    'Numbers: 0-9',
                    'Special: . (period), - (hyphen), _ (underscore)',
                    'No spaces or other special characters'
                ]
            },
            format: {
                title: 'Format Rules',
                rules: [
                    'Cannot start or end with . or -',
                    'Cannot contain consecutive dots (..) or hyphens (--)',
                    'Case insensitive (Alice = alice)'
                ]
            },
            quality: {
                title: 'Quality Guidelines',
                rules: [
                    'Avoid confusing characters (o vs 0, l vs 1)',
                    'Avoid homograph attacks (а vs a)',
                    'Pronounceable names score higher',
                    'Common words get quality bonus'
                ]
            },
            costs: {
                title: 'Registration Costs',
                rules: [
                    '1-4 characters: Premium pricing (10x base)',
                    '5-6 characters: High pricing (3x base)',
                    '7-10 characters: Standard pricing (1.5x base)',
                    '11+ characters: Base pricing (1x base)'
                ]
            }
        };
    }
}

// Export for use in main app
export default DPNSValidator;