/*
Language: Githook
Author: Florian Scholz
Description: Syntax highlighting for Githook (.ghook) files
Category: config
*/

export default function(hljs) {
  const KEYWORDS = [
    'run',
    'block',
    'allow',
    'when',
    'else',
    'match',
    'foreach',
    'parallel',
    'group',
    'macro',
    'use',
    'import',
    'let',
    'warn_if',
    'block_if',
    'in',
    'as',
    'matching',
    'message'
  ];

  const BUILT_IN = [
    'staged_files',
    'all_files',
    'modified_files',
    'branch_name',
    'file_size',
    'extension',
    'basename',
    'dirname',
    'commit_message',
    'content',
    'diff',
    'env',
    'modified_lines',
    'files_changed'
  ];

  const OPERATORS = [
    'and',
    'or',
    'not',
    'contains',
    'matches'
  ];

  return {
    name: 'Githook',
    aliases: ['ghook'],
    case_insensitive: false,
    keywords: {
      keyword: KEYWORDS,
      built_in: BUILT_IN,
      operator: OPERATORS
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
        className: 'string',
        begin: /\{[a-zA-Z_][a-zA-Z0-9_:]*\}/,
        relevance: 5
      },
      {
        className: 'operator',
        begin: /==|!=|>=|<=|>|</,
        relevance: 0
      }
    ]
  };
}
