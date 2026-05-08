#!/usr/bin/env node
/**
 * MCP Server for org-roam operations.
 *
 * Delegates all elisp execution to @org-roam-toolkit/emacs, which auto-loads
 * the `org-roam-skill` elisp package via the shared emacs-eval wrapper.
 */

import { Server } from '@modelcontextprotocol/sdk/server/index.js';
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js';
import {
  CallToolRequestSchema,
  ListToolsRequestSchema,
  ListResourcesRequestSchema,
  ReadResourceRequestSchema,
  type Tool,
} from '@modelcontextprotocol/sdk/types.js';
import {
  evalElisp as evalElispRaw,
  buildKeywordArgs,
  isDaemonRunning,
  quoteElispString,
} from '@org-roam-toolkit/emacs';
import { buildResources, toMcpResource } from './resources.js';

const PKG = 'org-roam-skill';

/** Local wrapper that always loads the org-roam-skill elisp package. */
function evalElisp(expr: string): unknown {
  return evalElispRaw(expr, { pkg: PKG });
}

/** Short alias for quoting a positional string argument. */
const q = quoteElispString;

const tools: Tool[] = [
  {
    name: 'roam_create_note',
    description:
      'Create a new org-roam note. Returns the file path of created note.',
    inputSchema: {
      type: 'object',
      properties: {
        title: { type: 'string', description: 'Note title (required)' },
        tags: {
          type: 'array',
          items: { type: 'string' },
          description: 'List of tags for the note',
        },
        content: {
          type: 'string',
          description: 'Initial content in org-mode format',
        },
        subdirectory: {
          type: 'string',
          enum: ['main', 'reference', 'projects', 'daily'],
          description: 'Subdirectory within org-roam-directory (default: main)',
        },
        sourceUrl: {
          type: 'string',
          description:
            'Original URL for reference notes (auto-generates References section)',
        },
        openArchive: {
          type: 'boolean',
          description:
            'Open archive.today submission in browser (default: true for reference notes)',
        },
        properties: {
          type: 'object',
          additionalProperties: { type: 'string' },
          description:
            'Additional PROPERTIES drawer entries as key-value pairs',
        },
      },
      required: ['title'],
    },
  },
  {
    name: 'roam_search_title',
    description:
      'Search org-roam notes by title (partial match). Returns list of [id, title, file] tuples.',
    inputSchema: {
      type: 'object',
      properties: {
        query: { type: 'string', description: 'Search term to match in note titles' },
      },
      required: ['query'],
    },
  },
  {
    name: 'roam_search_tag',
    description:
      'Search org-roam notes by tag. Returns list of [id, title, file, tags] tuples.',
    inputSchema: {
      type: 'object',
      properties: {
        tag: { type: 'string', description: 'Tag to search for' },
      },
      required: ['tag'],
    },
  },
  {
    name: 'roam_search_content',
    description:
      'Search org-roam notes by content (full-text). Returns list of [id, title, file] tuples.',
    inputSchema: {
      type: 'object',
      properties: {
        query: { type: 'string', description: 'Search term to find in note content' },
      },
      required: ['query'],
    },
  },
  {
    name: 'roam_get_backlinks',
    description:
      'Get notes that link TO the specified note. Returns list of [id, title, file] tuples.',
    inputSchema: {
      type: 'object',
      properties: {
        title: { type: 'string', description: 'Title of the note to find backlinks for' },
      },
      required: ['title'],
    },
  },
  {
    name: 'roam_create_link',
    description: 'Create links between two notes. Can create bidirectional links.',
    inputSchema: {
      type: 'object',
      properties: {
        source: { type: 'string', description: 'Title of the source note' },
        target: { type: 'string', description: 'Title of the target note to link to' },
        bidirectional: {
          type: 'boolean',
          description: 'Create links in both directions (default: false)',
        },
      },
      required: ['source', 'target'],
    },
  },
  {
    name: 'roam_add_reading_history',
    description:
      'Add an entry to the quarterly reading history log. NOT an org-roam node.',
    inputSchema: {
      type: 'object',
      properties: {
        title: { type: 'string', description: 'Article title' },
        url: { type: 'string', description: 'Source URL' },
        tags: { type: 'array', items: { type: 'string' }, description: 'Classification tags' },
        source: {
          type: 'string',
          description: 'Website name (e.g., cnblogs, github)',
        },
        author: { type: 'string', description: 'Author name' },
        summary: { type: 'string', description: 'One-line summary' },
        points: {
          type: 'array',
          items: { type: 'string' },
          description: 'Key points from the article',
        },
        rating: {
          type: 'number',
          minimum: 1,
          maximum: 5,
          description: 'Rating 1-5',
        },
      },
      required: ['title', 'url'],
    },
  },
  {
    name: 'roam_add_toolkit',
    description:
      'Add a resource to the quarterly toolkit collection. NOT an org-roam node.',
    inputSchema: {
      type: 'object',
      properties: {
        title: { type: 'string', description: 'Resource name' },
        url: { type: 'string', description: 'Resource URL' },
        tags: { type: 'array', items: { type: 'string' }, description: 'Classification tags' },
        category: {
          type: 'string',
          enum: ['library', 'tool', 'service', 'api'],
          description: 'Resource type',
        },
        description: { type: 'string', description: 'One-line description' },
      },
      required: ['title', 'url'],
    },
  },
  {
    name: 'roam_add_to_read',
    description:
      'Add a TODO item to read later under * Inbox in the read-later file (controlled by `org-roam-skill-to-read-file`; defaults to todo.org alongside org-roam-directory).',
    inputSchema: {
      type: 'object',
      properties: {
        title: { type: 'string', description: 'Article title' },
        url: { type: 'string', description: 'Link to read later' },
        summary: {
          type: 'string',
          description: 'Brief description of what it is about',
        },
      },
      required: ['title', 'url'],
    },
  },
  {
    name: 'roam_list_tags',
    description: 'List all unique tags across all org-roam notes.',
    inputSchema: { type: 'object', properties: {} },
  },
  {
    name: 'roam_doctor',
    description: 'Run comprehensive diagnostic check of org-roam setup.',
    inputSchema: { type: 'object', properties: {} },
  },
];

// ---------- Handlers ----------

function handleCreateNote(args: Record<string, unknown>): string {
  const title = args.title as string;
  const kwArgs: Record<string, unknown> = {};
  if (args.tags) kwArgs.tags = args.tags;
  if (args.content) kwArgs.content = args.content;
  if (args.subdirectory) kwArgs.subdirectory = args.subdirectory;
  if (args.sourceUrl) kwArgs['source-url'] = args.sourceUrl;
  if (args.openArchive !== undefined) kwArgs['open-archive'] = args.openArchive;
  if (args.properties) kwArgs.properties = args.properties;

  const kwString = buildKeywordArgs(kwArgs);
  const expr = `(org-roam-skill-create-note ${q(title)}${kwString ? ' ' + kwString : ''})`;
  return String(evalElisp(expr));
}

function handleSearchTitle(args: Record<string, unknown>): string {
  return String(evalElisp(`(org-roam-skill-search-by-title ${q(args.query as string)})`));
}

function handleSearchTag(args: Record<string, unknown>): string {
  return String(evalElisp(`(org-roam-skill-search-by-tag ${q(args.tag as string)})`));
}

function handleSearchContent(args: Record<string, unknown>): string {
  return String(evalElisp(`(org-roam-skill-search-by-content ${q(args.query as string)})`));
}

function handleGetBacklinks(args: Record<string, unknown>): string {
  return String(evalElisp(`(org-roam-skill-get-backlinks-by-title ${q(args.title as string)})`));
}

function handleCreateLink(args: Record<string, unknown>): string {
  const source = args.source as string;
  const target = args.target as string;
  const fn = args.bidirectional
    ? 'org-roam-skill-create-bidirectional-link'
    : 'org-roam-skill-insert-link-in-note';
  return String(evalElisp(`(${fn} ${q(source)} ${q(target)})`));
}

function handleAddReadingHistory(args: Record<string, unknown>): string {
  const title = args.title as string;
  const url = args.url as string;
  const kwArgs: Record<string, unknown> = {};
  if (args.tags) kwArgs.tags = args.tags;
  if (args.source) kwArgs.source = args.source;
  if (args.author) kwArgs.author = args.author;
  if (args.summary) kwArgs.summary = args.summary;
  if (args.points) kwArgs.points = args.points;
  if (args.rating) kwArgs.rating = args.rating;

  const kwString = buildKeywordArgs(kwArgs);
  const expr = `(org-roam-skill-add-reading-history ${q(title)} ${q(url)}${kwString ? ' ' + kwString : ''})`;
  return String(evalElisp(expr));
}

function handleAddToolkit(args: Record<string, unknown>): string {
  const title = args.title as string;
  const url = args.url as string;
  const kwArgs: Record<string, unknown> = {};
  if (args.tags) kwArgs.tags = args.tags;
  if (args.category) kwArgs.category = args.category;
  if (args.description) kwArgs.description = args.description;

  const kwString = buildKeywordArgs(kwArgs);
  const expr = `(org-roam-skill-add-toolkit-resource ${q(title)} ${q(url)}${kwString ? ' ' + kwString : ''})`;
  return String(evalElisp(expr));
}

function handleAddToRead(args: Record<string, unknown>): string {
  const title = args.title as string;
  const url = args.url as string;
  const kwArgs: Record<string, unknown> = {};
  if (args.summary) kwArgs.summary = args.summary;

  const kwString = buildKeywordArgs(kwArgs);
  const expr = `(org-roam-skill-add-to-read ${q(title)} ${q(url)}${kwString ? ' ' + kwString : ''})`;
  return String(evalElisp(expr));
}

function handleListTags(): string {
  return String(evalElisp('(org-roam-skill-list-all-tags)'));
}

function handleDoctor(): string {
  return String(evalElisp('(org-roam-doctor)'));
}

// ---------- Server ----------

const SERVER_INFO = { name: 'org-roam', version: '0.1.0' } as const;

const server = new Server(SERVER_INFO, {
  capabilities: { tools: {}, resources: {} },
});

const resources = buildResources({
  serverName: SERVER_INFO.name,
  serverVersion: SERVER_INFO.version,
  toolCount: tools.length,
});
const resourcesByUri = new Map(resources.map((r) => [r.uri, r]));

server.setRequestHandler(ListToolsRequestSchema, async () => ({ tools }));

server.setRequestHandler(ListResourcesRequestSchema, async () => ({
  resources: resources.map(toMcpResource),
}));

server.setRequestHandler(ReadResourceRequestSchema, async (request) => {
  const def = resourcesByUri.get(request.params.uri);
  if (!def) {
    throw new Error(`Unknown resource URI: ${request.params.uri}`);
  }
  const probe = await def.read();
  return {
    contents: [
      {
        uri: def.uri,
        mimeType: 'application/json',
        text: JSON.stringify(probe),
      },
    ],
  };
});

server.setRequestHandler(CallToolRequestSchema, async (request) => {
  const { name, arguments: args } = request.params;

  if (!isDaemonRunning()) {
    return {
      content: [
        {
          type: 'text',
          text: 'Error: Emacs daemon is not running. Start it with: emacs --daemon',
        },
      ],
      isError: true,
    };
  }

  try {
    let result: string;
    const a = (args ?? {}) as Record<string, unknown>;

    switch (name) {
      case 'roam_create_note':         result = handleCreateNote(a); break;
      case 'roam_search_title':        result = handleSearchTitle(a); break;
      case 'roam_search_tag':          result = handleSearchTag(a); break;
      case 'roam_search_content':      result = handleSearchContent(a); break;
      case 'roam_get_backlinks':       result = handleGetBacklinks(a); break;
      case 'roam_create_link':         result = handleCreateLink(a); break;
      case 'roam_add_reading_history': result = handleAddReadingHistory(a); break;
      case 'roam_add_toolkit':         result = handleAddToolkit(a); break;
      case 'roam_add_to_read':         result = handleAddToRead(a); break;
      case 'roam_list_tags':           result = handleListTags(); break;
      case 'roam_doctor':              result = handleDoctor(); break;
      default:
        return {
          content: [{ type: 'text', text: `Unknown tool: ${name}` }],
          isError: true,
        };
    }

    return { content: [{ type: 'text', text: result }] };
  } catch (error) {
    const msg = error instanceof Error ? error.message : String(error);
    return {
      content: [{ type: 'text', text: `Error: ${msg}` }],
      isError: true,
    };
  }
});

async function main() {
  const transport = new StdioServerTransport();
  await server.connect(transport);
  console.error('org-roam MCP server running on stdio');
}

main().catch((error) => {
  console.error('Fatal error:', error);
  process.exit(1);
});
