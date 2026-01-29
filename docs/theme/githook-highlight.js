// Register Githook language with highlight.js
(function() {
    'use strict';
    
    function hljsDefineGithook(hljs) {
        return {
            name: 'Githook',
            aliases: ['ghook'],
            case_insensitive: false,
            keywords: {
                keyword: 'run block allow when else match foreach parallel group macro use import let warn_if block_if in as matching message',
                built_in: 'staged_files all_files modified_files branch_name file_size extension basename dirname commit_message content diff env modified_lines files_changed',
                literal: 'true false'
            },
            contains: [
                hljs.COMMENT('#', '$'),
                hljs.QUOTE_STRING_MODE,
                hljs.NUMBER_MODE,
                {
                    className: 'meta',
                    begin: '@[a-zA-Z_][a-zA-Z0-9_]*',
                    relevance: 10
                },
                {
                    className: 'variable',
                    begin: /\{[a-zA-Z_][a-zA-Z0-9_:]*\}/,
                    relevance: 5
                },
                {
                    className: 'operator',
                    begin: /(==|!=|>=|<=|>|<|and|or|not|contains|matches)/,
                    relevance: 0
                }
            ]
        };
    }
    
    // Register when highlight.js is loaded
    if (typeof hljs !== 'undefined') {
        hljs.registerLanguage('githook', hljsDefineGithook);
        hljs.registerLanguage('ghook', hljsDefineGithook);
    }
    
    // Also register on document ready for mdBook
    document.addEventListener('DOMContentLoaded', function() {
        if (typeof hljs !== 'undefined') {
            hljs.registerLanguage('githook', hljsDefineGithook);
            hljs.registerLanguage('ghook', hljsDefineGithook);
            // Re-highlight all code blocks
            document.querySelectorAll('pre code.language-githook, pre code.language-ghook').forEach(function(block) {
                hljs.highlightElement(block);
            });
        }
    });
})();
