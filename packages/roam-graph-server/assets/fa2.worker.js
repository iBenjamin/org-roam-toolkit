/**
 * ForceAtlas2 synchronous layout — IIFE build
 *
 * Source: graphology-layout-forceatlas2@0.10.1 (index.js + iterate.js +
 *         helpers.js + defaults.js) with graphology-utils@2.5.1 inlined.
 *
 * Registers window.forceAtlas2 = synchronousLayout (function + .assign method).
 * No Worker, no CommonJS — plain browser IIFE.
 *
 * API used by app.js:
 *   forceAtlas2.assign(graph, { iterations: 200, settings: { ... } });
 */
(function (global) {
  'use strict';

  // ── graphology-utils/is-graph ──────────────────────────────────────────────
  function isGraph(value) {
    return (
      value !== null &&
      typeof value === 'object' &&
      typeof value.addUndirectedEdgeWithKey === 'function' &&
      typeof value.dropNode === 'function' &&
      typeof value.multi === 'boolean'
    );
  }

  // ── graphology-utils/getters (createEdgeWeightGetter only) ─────────────────
  function coerceWeight(value) {
    if (typeof value !== 'number' || isNaN(value)) return 1;
    return value;
  }

  function createEdgeValueGetter(nameOrFunction, defaultValue) {
    var getter = {};

    var coerceToDefault = function (v) {
      if (typeof v === 'undefined') return defaultValue;
      return v;
    };

    if (typeof defaultValue === 'function') coerceToDefault = defaultValue;

    var get = function (attributes) {
      return coerceToDefault(attributes[nameOrFunction]);
    };

    var returnDefault = function () {
      return coerceToDefault(undefined);
    };

    if (typeof nameOrFunction === 'string') {
      getter.fromAttributes = get;
      getter.fromGraph = function (graph, edge) {
        return get(graph.getEdgeAttributes(edge));
      };
      getter.fromEntry = function (edge, attributes) {
        return get(attributes);
      };
      getter.fromPartialEntry = getter.fromEntry;
      getter.fromMinimalEntry = getter.fromEntry;
    } else if (typeof nameOrFunction === 'function') {
      getter.fromAttributes = function () {
        throw new Error('graphology-utils/getters: irrelevant usage.');
      };
      getter.fromGraph = function (graph, edge) {
        var extremities = graph.extremities(edge);
        return coerceToDefault(
          nameOrFunction(
            edge,
            graph.getEdgeAttributes(edge),
            extremities[0],
            extremities[1],
            graph.getNodeAttributes(extremities[0]),
            graph.getNodeAttributes(extremities[1]),
            graph.isUndirected(edge)
          )
        );
      };
      getter.fromEntry = function (e, a, s, t, sa, ta, u) {
        return coerceToDefault(nameOrFunction(e, a, s, t, sa, ta, u));
      };
      getter.fromPartialEntry = function (e, a, s, t) {
        return coerceToDefault(nameOrFunction(e, a, s, t));
      };
      getter.fromMinimalEntry = function (e, a) {
        return coerceToDefault(nameOrFunction(e, a));
      };
    } else {
      getter.fromAttributes = returnDefault;
      getter.fromGraph = returnDefault;
      getter.fromEntry = returnDefault;
      getter.fromMinimalEntry = returnDefault;
    }

    return getter;
  }

  function createEdgeWeightGetter(name) {
    return createEdgeValueGetter(name, coerceWeight);
  }

  // ── graphology-layout-forceatlas2/defaults.js ──────────────────────────────
  var DEFAULT_SETTINGS = {
    linLogMode: false,
    outboundAttractionDistribution: false,
    adjustSizes: false,
    edgeWeightInfluence: 1,
    scalingRatio: 1,
    strongGravityMode: false,
    gravity: 1,
    slowDown: 1,
    barnesHutOptimize: false,
    barnesHutTheta: 0.5
  };

  // ── graphology-layout-forceatlas2/helpers.js ───────────────────────────────
  var PPN = 10;
  var PPE = 3;

  var helpers = {};

  helpers.assign = function (target) {
    target = target || {};
    var objects = Array.prototype.slice.call(arguments).slice(1), i, k, l;
    for (i = 0, l = objects.length; i < l; i++) {
      if (!objects[i]) continue;
      for (k in objects[i]) target[k] = objects[i][k];
    }
    return target;
  };

  helpers.validateSettings = function (settings) {
    if ('linLogMode' in settings && typeof settings.linLogMode !== 'boolean')
      return {message: 'the `linLogMode` setting should be a boolean.'};
    if ('outboundAttractionDistribution' in settings && typeof settings.outboundAttractionDistribution !== 'boolean')
      return {message: 'the `outboundAttractionDistribution` setting should be a boolean.'};
    if ('adjustSizes' in settings && typeof settings.adjustSizes !== 'boolean')
      return {message: 'the `adjustSizes` setting should be a boolean.'};
    if ('edgeWeightInfluence' in settings && typeof settings.edgeWeightInfluence !== 'number')
      return {message: 'the `edgeWeightInfluence` setting should be a number.'};
    if ('scalingRatio' in settings && !(typeof settings.scalingRatio === 'number' && settings.scalingRatio >= 0))
      return {message: 'the `scalingRatio` setting should be a number >= 0.'};
    if ('strongGravityMode' in settings && typeof settings.strongGravityMode !== 'boolean')
      return {message: 'the `strongGravityMode` setting should be a boolean.'};
    if ('gravity' in settings && !(typeof settings.gravity === 'number' && settings.gravity >= 0))
      return {message: 'the `gravity` setting should be a number >= 0.'};
    if ('slowDown' in settings && !(typeof settings.slowDown === 'number' || settings.slowDown >= 0))
      return {message: 'the `slowDown` setting should be a number >= 0.'};
    if ('barnesHutOptimize' in settings && typeof settings.barnesHutOptimize !== 'boolean')
      return {message: 'the `barnesHutOptimize` setting should be a boolean.'};
    if ('barnesHutTheta' in settings && !(typeof settings.barnesHutTheta === 'number' && settings.barnesHutTheta >= 0))
      return {message: 'the `barnesHutTheta` setting should be a number >= 0.'};
    return null;
  };

  helpers.graphToByteArrays = function (graph, getEdgeWeight) {
    var order = graph.order;
    var size = graph.size;
    var index = {};
    var j;

    var NodeMatrix = new Float32Array(order * PPN);
    var EdgeMatrix = new Float32Array(size * PPE);

    j = 0;
    graph.forEachNode(function (node, attr) {
      index[node] = j;
      NodeMatrix[j]     = attr.x;
      NodeMatrix[j + 1] = attr.y;
      NodeMatrix[j + 2] = 0; // dx
      NodeMatrix[j + 3] = 0; // dy
      NodeMatrix[j + 4] = 0; // old_dx
      NodeMatrix[j + 5] = 0; // old_dy
      NodeMatrix[j + 6] = 1; // mass
      NodeMatrix[j + 7] = 1; // convergence
      NodeMatrix[j + 8] = attr.size || 1;
      NodeMatrix[j + 9] = attr.fixed ? 1 : 0;
      j += PPN;
    });

    j = 0;
    graph.forEachEdge(function (edge, attr, source, target, sa, ta, u) {
      var sj = index[source];
      var tj = index[target];
      var weight = getEdgeWeight(edge, attr, source, target, sa, ta, u);
      NodeMatrix[sj + 6] += weight;
      NodeMatrix[tj + 6] += weight;
      EdgeMatrix[j]     = sj;
      EdgeMatrix[j + 1] = tj;
      EdgeMatrix[j + 2] = weight;
      j += PPE;
    });

    return {nodes: NodeMatrix, edges: EdgeMatrix};
  };

  helpers.assignLayoutChanges = function (graph, NodeMatrix, outputReducer) {
    var i = 0;
    graph.updateEachNodeAttributes(function (node, attr) {
      attr.x = NodeMatrix[i];
      attr.y = NodeMatrix[i + 1];
      i += PPN;
      return outputReducer ? outputReducer(node, attr) : attr;
    });
  };

  helpers.collectLayoutChanges = function (graph, NodeMatrix, outputReducer) {
    var nodes = graph.nodes(), positions = {};
    for (var i = 0, j = 0, l = NodeMatrix.length; i < l; i += PPN) {
      if (outputReducer) {
        var newAttr = Object.assign({}, graph.getNodeAttributes(nodes[j]));
        newAttr.x = NodeMatrix[i];
        newAttr.y = NodeMatrix[i + 1];
        newAttr = outputReducer(nodes[j], newAttr);
        positions[nodes[j]] = {x: newAttr.x, y: newAttr.y};
      } else {
        positions[nodes[j]] = {x: NodeMatrix[i], y: NodeMatrix[i + 1]};
      }
      j++;
    }
    return positions;
  };

  // ── graphology-layout-forceatlas2/iterate.js ───────────────────────────────
  /* eslint no-constant-condition: 0 */
  var NODE_X = 0, NODE_Y = 1, NODE_DX = 2, NODE_DY = 3;
  var NODE_OLD_DX = 4, NODE_OLD_DY = 5;
  var NODE_MASS = 6, NODE_CONVERGENCE = 7, NODE_SIZE = 8, NODE_FIXED = 9;
  var EDGE_SOURCE = 0, EDGE_TARGET = 1, EDGE_WEIGHT = 2;
  var REGION_NODE = 0, REGION_CENTER_X = 1, REGION_CENTER_Y = 2;
  var REGION_SIZE = 3, REGION_NEXT_SIBLING = 4, REGION_FIRST_CHILD = 5;
  var REGION_MASS = 6, REGION_MASS_CENTER_X = 7, REGION_MASS_CENTER_Y = 8;
  var SUBDIVISION_ATTEMPTS = 3;
  var PPR = 9;
  var MAX_FORCE = 10;

  var iterate = function iterate(options, NodeMatrix, EdgeMatrix) {
    var l, r, n, n1, n2, rn, e, w, g, k, m;

    var order = NodeMatrix.length;
    var size  = EdgeMatrix.length;

    var adjustSizes = options.adjustSizes;

    var thetaSquared = Math.pow(options.barnesHutTheta, 2);

    var outboundAttCompensation, coefficient, xDist, yDist, ewc,
        distance, factor;

    // 1) Initialise layout iteration
    for (n = 0; n < order; n += PPN) {
      if (NodeMatrix[n + NODE_FIXED]) continue;
      NodeMatrix[n + NODE_OLD_DX] = NodeMatrix[n + NODE_DX];
      NodeMatrix[n + NODE_OLD_DY] = NodeMatrix[n + NODE_DY];
      NodeMatrix[n + NODE_DX]     = 0;
      NodeMatrix[n + NODE_DY]     = 0;
    }

    // 2) Outbound attraction compensation
    if (options.outboundAttractionDistribution) {
      outboundAttCompensation = 0;
      for (n = 0; n < order; n += PPN) {
        outboundAttCompensation += NodeMatrix[n + NODE_MASS];
      }
      outboundAttCompensation /= order / PPN;
    }

    // 3) Barnes-Hut or brute-force repulsion
    if (options.barnesHutOptimize) {
      var minX = Infinity, maxX = -Infinity, minY = Infinity, maxY = -Infinity,
          q, q1, q2, subdivisionAttempts;

      for (n = 0; n < order; n += PPN) {
        minX = Math.min(minX, NodeMatrix[n + NODE_X]);
        maxX = Math.max(maxX, NodeMatrix[n + NODE_X]);
        minY = Math.min(minY, NodeMatrix[n + NODE_Y]);
        maxY = Math.max(maxY, NodeMatrix[n + NODE_Y]);
      }

      var centerX = (minX + maxX) / 2, centerY = (minY + maxY) / 2;
      var rootSize = Math.max(maxX - minX, maxY - minY);
      var RegionMatrix = new Float32Array((order / PPN * 4 + 1) * PPR);

      RegionMatrix[0 + REGION_NODE]          = -1;
      RegionMatrix[0 + REGION_CENTER_X]      = centerX;
      RegionMatrix[0 + REGION_CENTER_Y]      = centerY;
      RegionMatrix[0 + REGION_SIZE]          = rootSize;
      RegionMatrix[0 + REGION_NEXT_SIBLING]  = -1;
      RegionMatrix[0 + REGION_FIRST_CHILD]   = -1;
      RegionMatrix[0 + REGION_MASS]          = 0;
      RegionMatrix[0 + REGION_MASS_CENTER_X] = 0;
      RegionMatrix[0 + REGION_MASS_CENTER_Y] = 0;

      l = PPR;

      for (n = 0; n < order; n += PPN) {
        rn = 0;

        while (true) {
          if (RegionMatrix[rn + REGION_FIRST_CHILD] < 0) {
            // rn is a leaf
            if (RegionMatrix[rn + REGION_NODE] < 0) {
              // Empty region: store n here
              RegionMatrix[rn + REGION_NODE] = n;
              break;
            } else {
              // Occupied leaf: subdivide
              subdivisionAttempts = SUBDIVISION_ATTEMPTS;

              while (RegionMatrix[rn + REGION_FIRST_CHILD] < 0) {
                var regionSize = RegionMatrix[rn + REGION_SIZE] / 2;
                var regionCenterX = RegionMatrix[rn + REGION_CENTER_X];
                var regionCenterY = RegionMatrix[rn + REGION_CENTER_Y];

                if (l + 4 * PPR > RegionMatrix.length) {
                  var newRegionMatrix = new Float32Array(RegionMatrix.length + order * PPR);
                  newRegionMatrix.set(RegionMatrix);
                  RegionMatrix = newRegionMatrix;
                }

                for (q = 0; q < 4; q++) {
                  RegionMatrix[l + q * PPR + REGION_NODE]          = -1;
                  RegionMatrix[l + q * PPR + REGION_SIZE]          = regionSize;
                  RegionMatrix[l + q * PPR + REGION_NEXT_SIBLING]  = l + (q + 1) * PPR;
                  RegionMatrix[l + q * PPR + REGION_FIRST_CHILD]   = -1;
                  RegionMatrix[l + q * PPR + REGION_MASS]          = 0;
                  RegionMatrix[l + q * PPR + REGION_MASS_CENTER_X] = 0;
                  RegionMatrix[l + q * PPR + REGION_MASS_CENTER_Y] = 0;
                }
                RegionMatrix[l + 3 * PPR + REGION_NEXT_SIBLING] = RegionMatrix[rn + REGION_NEXT_SIBLING];

                q1 = RegionMatrix[rn + REGION_NODE];

                RegionMatrix[l + REGION_CENTER_X]     = regionCenterX - regionSize / 2;
                RegionMatrix[l + REGION_CENTER_Y]     = regionCenterY - regionSize / 2;
                RegionMatrix[l + PPR + REGION_CENTER_X] = regionCenterX + regionSize / 2;
                RegionMatrix[l + PPR + REGION_CENTER_Y] = regionCenterY - regionSize / 2;
                RegionMatrix[l + 2 * PPR + REGION_CENTER_X] = regionCenterX - regionSize / 2;
                RegionMatrix[l + 2 * PPR + REGION_CENTER_Y] = regionCenterY + regionSize / 2;
                RegionMatrix[l + 3 * PPR + REGION_CENTER_X] = regionCenterX + regionSize / 2;
                RegionMatrix[l + 3 * PPR + REGION_CENTER_Y] = regionCenterY + regionSize / 2;

                RegionMatrix[rn + REGION_FIRST_CHILD] = l;
                l += 4 * PPR;

                // Insert q1 in correct sub-region
                var q1Region = RegionMatrix[rn + REGION_FIRST_CHILD];
                while (true) {
                  q2 = RegionMatrix[q1Region + REGION_NEXT_SIBLING];
                  if (q2 < 0 ||
                    (NodeMatrix[q1 + NODE_X] < RegionMatrix[q1Region + REGION_CENTER_X]) ===
                    (NodeMatrix[q1 + NODE_X] < RegionMatrix[RegionMatrix[rn + REGION_FIRST_CHILD] + REGION_CENTER_X]) &&
                    (NodeMatrix[q1 + NODE_Y] < RegionMatrix[q1Region + REGION_CENTER_Y]) ===
                    (NodeMatrix[q1 + NODE_Y] < RegionMatrix[RegionMatrix[rn + REGION_FIRST_CHILD] + REGION_CENTER_Y])) {
                    break;
                  }
                  q1Region = q2;
                }
                RegionMatrix[q1Region + REGION_NODE] = q1;

                if (--subdivisionAttempts <= 0) {
                  RegionMatrix[rn + REGION_NODE] = n;
                  break;
                }
              }

              if (RegionMatrix[rn + REGION_FIRST_CHILD] < 0) break;

              // Find sub-region for n
              rn = RegionMatrix[rn + REGION_FIRST_CHILD];
              while (true) {
                q2 = RegionMatrix[rn + REGION_NEXT_SIBLING];
                if (q2 < 0 ||
                  (NodeMatrix[n + NODE_X] < RegionMatrix[rn + REGION_CENTER_X]) !==
                  (NodeMatrix[n + NODE_X] >= RegionMatrix[RegionMatrix[RegionMatrix[rn - PPR] ? rn : 0 + REGION_FIRST_CHILD] + REGION_CENTER_X])) {
                  break;
                }
                rn = q2;
              }
              break;
            }
          } else {
            // rn has children
            RegionMatrix[rn + REGION_MASS] += NodeMatrix[n + NODE_MASS];
            RegionMatrix[rn + REGION_MASS_CENTER_X] +=
              NodeMatrix[n + NODE_MASS] * NodeMatrix[n + NODE_X];
            RegionMatrix[rn + REGION_MASS_CENTER_Y] +=
              NodeMatrix[n + NODE_MASS] * NodeMatrix[n + NODE_Y];

            // Find correct child
            rn = RegionMatrix[rn + REGION_FIRST_CHILD];
            while (true) {
              q2 = RegionMatrix[rn + REGION_NEXT_SIBLING];
              if (q2 < 0) break;
              if ((NodeMatrix[n + NODE_X] < RegionMatrix[rn + REGION_CENTER_X]) !==
                  (NodeMatrix[n + NODE_X] < RegionMatrix[q2 + REGION_CENTER_X])) {
                break;
              }
              if ((NodeMatrix[n + NODE_Y] < RegionMatrix[rn + REGION_CENTER_Y]) !==
                  (NodeMatrix[n + NODE_Y] < RegionMatrix[q2 + REGION_CENTER_Y])) {
                break;
              }
              rn = q2;
            }
          }
        }
      }

      // Normalise mass centers
      for (rn = 0; rn < l; rn += PPR) {
        if (RegionMatrix[rn + REGION_MASS] > 0) {
          RegionMatrix[rn + REGION_MASS_CENTER_X] /= RegionMatrix[rn + REGION_MASS];
          RegionMatrix[rn + REGION_MASS_CENTER_Y] /= RegionMatrix[rn + REGION_MASS];
        }
      }

      // Apply Barnes-Hut repulsion
      for (n = 0; n < order; n += PPN) {
        if (NodeMatrix[n + NODE_FIXED]) continue;

        rn = 0;
        while (true) {
          if (RegionMatrix[rn + REGION_FIRST_CHILD] >= 0) {
            xDist = NodeMatrix[n + NODE_X] - RegionMatrix[rn + REGION_MASS_CENTER_X];
            yDist = NodeMatrix[n + NODE_Y] - RegionMatrix[rn + REGION_MASS_CENTER_Y];
            distance = Math.sqrt(xDist * xDist + yDist * yDist);

            if (distance * distance / RegionMatrix[rn + REGION_MASS] < thetaSquared) {
              // Treat region as single body
              if (distance > 0) {
                coefficient = options.scalingRatio * NodeMatrix[n + NODE_MASS] *
                  RegionMatrix[rn + REGION_MASS] / distance / distance;
                NodeMatrix[n + NODE_DX] += coefficient * xDist / distance;
                NodeMatrix[n + NODE_DY] += coefficient * yDist / distance;
              }
              rn = RegionMatrix[rn + REGION_NEXT_SIBLING];
              if (rn < 0) break;
              continue;
            } else {
              rn = RegionMatrix[rn + REGION_FIRST_CHILD];
              continue;
            }
          } else {
            // Leaf region
            if (RegionMatrix[rn + REGION_NODE] >= 0 && RegionMatrix[rn + REGION_NODE] !== n) {
              n1 = RegionMatrix[rn + REGION_NODE];
              xDist = NodeMatrix[n + NODE_X] - NodeMatrix[n1 + NODE_X];
              yDist = NodeMatrix[n + NODE_Y] - NodeMatrix[n1 + NODE_Y];
              distance = Math.sqrt(xDist * xDist + yDist * yDist);

              if (distance > 0) {
                if (adjustSizes) {
                  ewc = distance - NodeMatrix[n + NODE_SIZE] - NodeMatrix[n1 + NODE_SIZE];
                  if (ewc > 0) {
                    coefficient = options.scalingRatio * NodeMatrix[n + NODE_MASS] *
                      NodeMatrix[n1 + NODE_MASS] / ewc / ewc;
                  } else {
                    coefficient = options.scalingRatio * 100 * NodeMatrix[n + NODE_MASS] *
                      NodeMatrix[n1 + NODE_MASS];
                  }
                } else {
                  coefficient = options.scalingRatio * NodeMatrix[n + NODE_MASS] *
                    NodeMatrix[n1 + NODE_MASS] / distance / distance;
                }
                NodeMatrix[n + NODE_DX] += coefficient * xDist / distance;
                NodeMatrix[n + NODE_DY] += coefficient * yDist / distance;
              }
            }
            rn = RegionMatrix[rn + REGION_NEXT_SIBLING];
            if (rn < 0) break;
          }
        }
      }
    } else {
      // Brute-force repulsion
      for (n1 = 0; n1 < order; n1 += PPN) {
        for (n2 = 0; n2 < n1; n2 += PPN) {
          xDist = NodeMatrix[n1 + NODE_X] - NodeMatrix[n2 + NODE_X];
          yDist = NodeMatrix[n1 + NODE_Y] - NodeMatrix[n2 + NODE_Y];
          distance = Math.sqrt(xDist * xDist + yDist * yDist);

          if (distance > 0) {
            if (adjustSizes) {
              ewc = distance - NodeMatrix[n1 + NODE_SIZE] - NodeMatrix[n2 + NODE_SIZE];
              if (ewc > 0) {
                coefficient = options.scalingRatio * NodeMatrix[n1 + NODE_MASS] *
                  NodeMatrix[n2 + NODE_MASS] / ewc / ewc;
              } else {
                coefficient = options.scalingRatio * 100 * NodeMatrix[n1 + NODE_MASS] *
                  NodeMatrix[n2 + NODE_MASS];
              }
            } else {
              coefficient = options.scalingRatio * NodeMatrix[n1 + NODE_MASS] *
                NodeMatrix[n2 + NODE_MASS] / distance / distance;
            }
            factor = coefficient / distance;
            NodeMatrix[n1 + NODE_DX] += factor * xDist;
            NodeMatrix[n1 + NODE_DY] += factor * yDist;
            NodeMatrix[n2 + NODE_DX] -= factor * xDist;
            NodeMatrix[n2 + NODE_DY] -= factor * yDist;
          }
        }
      }
    }

    // 4) Gravity
    g = options.gravity / options.scalingRatio;
    coefficient = options.scalingRatio;

    for (n = 0; n < order; n += PPN) {
      if (NodeMatrix[n + NODE_FIXED]) continue;

      xDist = NodeMatrix[n + NODE_X];
      yDist = NodeMatrix[n + NODE_Y];
      distance = Math.sqrt(xDist * xDist + yDist * yDist);

      if (distance > 0) {
        if (options.strongGravityMode) {
          factor = coefficient * NodeMatrix[n + NODE_MASS] * g;
        } else {
          factor = coefficient * NodeMatrix[n + NODE_MASS] * g / distance;
        }
        NodeMatrix[n + NODE_DX] -= factor * xDist;
        NodeMatrix[n + NODE_DY] -= factor * yDist;
      }
    }

    // 5) Attraction
    for (e = 0; e < size; e += PPE) {
      n1 = EdgeMatrix[e + EDGE_SOURCE];
      n2 = EdgeMatrix[e + EDGE_TARGET];
      w  = EdgeMatrix[e + EDGE_WEIGHT];

      xDist = NodeMatrix[n1 + NODE_X] - NodeMatrix[n2 + NODE_X];
      yDist = NodeMatrix[n1 + NODE_Y] - NodeMatrix[n2 + NODE_Y];
      distance = Math.sqrt(xDist * xDist + yDist * yDist);

      if (options.linLogMode) {
        if (options.outboundAttractionDistribution) {
          coefficient = -outboundAttCompensation * w / NodeMatrix[n1 + NODE_MASS];
        } else {
          coefficient = -outboundAttCompensation * w;
        }
        if (distance > 0) {
          factor = coefficient * Math.log(1 + distance) / distance;
        } else {
          continue;
        }
      } else {
        if (options.outboundAttractionDistribution) {
          coefficient = -outboundAttCompensation * w / NodeMatrix[n1 + NODE_MASS];
        } else {
          coefficient = -outboundAttCompensation * w;
        }
        if (adjustSizes) {
          if (distance > 0) {
            ewc = distance - NodeMatrix[n1 + NODE_SIZE] - NodeMatrix[n2 + NODE_SIZE];
            if (ewc <= 0) {
              factor = coefficient * 100;
            } else {
              factor = coefficient / ewc;
            }
          } else {
            factor = coefficient * 100;
          }
        } else {
          factor = coefficient;
        }
      }

      NodeMatrix[n1 + NODE_DX] += factor * xDist;
      NodeMatrix[n1 + NODE_DY] += factor * yDist;
      NodeMatrix[n2 + NODE_DX] -= factor * xDist;
      NodeMatrix[n2 + NODE_DY] -= factor * yDist;
    }

    // 6) Apply forces
    var totalSwinging = 0, totalEffectiveTraction = 0, swinging;

    for (n = 0; n < order; n += PPN) {
      if (NodeMatrix[n + NODE_FIXED]) continue;

      swinging = Math.sqrt(
        (NodeMatrix[n + NODE_OLD_DX] - NodeMatrix[n + NODE_DX]) *
        (NodeMatrix[n + NODE_OLD_DX] - NodeMatrix[n + NODE_DX]) +
        (NodeMatrix[n + NODE_OLD_DY] - NodeMatrix[n + NODE_DY]) *
        (NodeMatrix[n + NODE_OLD_DY] - NodeMatrix[n + NODE_DY])
      );

      totalSwinging += NodeMatrix[n + NODE_MASS] * swinging;
      totalEffectiveTraction += NodeMatrix[n + NODE_MASS] * 0.5 *
        Math.sqrt(
          (NodeMatrix[n + NODE_OLD_DX] + NodeMatrix[n + NODE_DX]) *
          (NodeMatrix[n + NODE_OLD_DX] + NodeMatrix[n + NODE_DX]) +
          (NodeMatrix[n + NODE_OLD_DY] + NodeMatrix[n + NODE_DY]) *
          (NodeMatrix[n + NODE_OLD_DY] + NodeMatrix[n + NODE_DY])
        );
    }

    var estimatedOptimalJitterTolerance = 0.05 * Math.sqrt(order / PPN);
    var minJT = Math.sqrt(estimatedOptimalJitterTolerance);
    var maxJT = 10;
    var jt = options.jitterTolerance *
      Math.max(minJT,
        Math.min(maxJT,
          estimatedOptimalJitterTolerance *
          totalEffectiveTraction / Math.pow(order / PPN, 2)));

    var minSpeedEfficiency = 0.05;

    var globalSpeed, speedEfficiency;
    if (totalSwinging / totalEffectiveTraction > 2.0) {
      if (!options._speedEfficiency) options._speedEfficiency = 1;
      options._speedEfficiency *= 0.5;
      if (options._speedEfficiency < minSpeedEfficiency) options._speedEfficiency = minSpeedEfficiency;
    }
    speedEfficiency = options._speedEfficiency || 1;

    if (totalSwinging === 0) {
      globalSpeed = Infinity;
    } else {
      globalSpeed = jt * speedEfficiency * totalEffectiveTraction / totalSwinging;
    }

    var maxRise = 0.5;
    if (options._speed) {
      if (globalSpeed > options._speed * (1 + maxRise)) {
        globalSpeed = options._speed * (1 + maxRise);
      }
    }
    options._speed = globalSpeed;

    var newX, newY, nodespeed;

    for (n = 0; n < order; n += PPN) {
      if (NodeMatrix[n + NODE_FIXED]) continue;

      swinging = NodeMatrix[n + NODE_MASS] * Math.sqrt(
        (NodeMatrix[n + NODE_OLD_DX] - NodeMatrix[n + NODE_DX]) *
        (NodeMatrix[n + NODE_OLD_DX] - NodeMatrix[n + NODE_DX]) +
        (NodeMatrix[n + NODE_OLD_DY] - NodeMatrix[n + NODE_DY]) *
        (NodeMatrix[n + NODE_OLD_DY] - NodeMatrix[n + NODE_DY])
      );

      nodespeed = 0.1 * globalSpeed / (1 + globalSpeed * Math.sqrt(swinging));

      var df = Math.sqrt(
        Math.pow(NodeMatrix[n + NODE_DX], 2) +
        Math.pow(NodeMatrix[n + NODE_DY], 2)
      );

      if (df > MAX_FORCE / nodespeed) {
        nodespeed = MAX_FORCE / df;
      }

      NodeMatrix[n + NODE_CONVERGENCE] = Math.min(
        1,
        Math.sqrt(
          (nodespeed * (Math.pow(NodeMatrix[n + NODE_DX], 2) + Math.pow(NodeMatrix[n + NODE_DY], 2))) /
          (1 + Math.sqrt(swinging))
        )
      );

      newX = NodeMatrix[n + NODE_X] + NodeMatrix[n + NODE_DX] * (nodespeed / options.slowDown);
      NodeMatrix[n + NODE_X] = newX;
      newY = NodeMatrix[n + NODE_Y] + NodeMatrix[n + NODE_DY] * (nodespeed / options.slowDown);
      NodeMatrix[n + NODE_Y] = newY;
    }

    return {};
  };

  // ── graphology-layout-forceatlas2/index.js ─────────────────────────────────
  function abstractSynchronousLayout(assign, graph, params) {
    if (!isGraph(graph))
      throw new Error(
        'graphology-layout-forceatlas2: the given graph is not a valid graphology instance.'
      );

    if (typeof params === 'number') params = {iterations: params};

    var iterations = params.iterations;

    if (typeof iterations !== 'number')
      throw new Error(
        'graphology-layout-forceatlas2: invalid number of iterations.'
      );

    if (iterations <= 0)
      throw new Error(
        'graphology-layout-forceatlas2: you should provide a positive number of iterations.'
      );

    var getEdgeWeight = createEdgeWeightGetter(
      'getEdgeWeight' in params ? params.getEdgeWeight : 'weight'
    ).fromEntry;

    var outputReducer =
      typeof params.outputReducer === 'function' ? params.outputReducer : null;

    var settings = helpers.assign({}, DEFAULT_SETTINGS, params.settings);
    // jitterTolerance is not in defaults but iterate uses it
    if (!('jitterTolerance' in settings)) settings.jitterTolerance = 1;

    var validationError = helpers.validateSettings(settings);
    if (validationError)
      throw new Error('graphology-layout-forceatlas2: ' + validationError.message);

    var matrices = helpers.graphToByteArrays(graph, getEdgeWeight);

    var i;
    for (i = 0; i < iterations; i++)
      iterate(settings, matrices.nodes, matrices.edges);

    if (assign) {
      helpers.assignLayoutChanges(graph, matrices.nodes, outputReducer);
      return;
    }

    return helpers.collectLayoutChanges(graph, matrices.nodes);
  }

  function inferSettings(graph) {
    var order = typeof graph === 'number' ? graph : graph.order;
    return {
      barnesHutOptimize: order > 2000,
      strongGravityMode: true,
      gravity: 0.05,
      scalingRatio: 10,
      slowDown: 1 + Math.log(order)
    };
  }

  var synchronousLayout = abstractSynchronousLayout.bind(null, false);
  synchronousLayout.assign    = abstractSynchronousLayout.bind(null, true);
  synchronousLayout.inferSettings = inferSettings;

  // ── Register global ────────────────────────────────────────────────────────
  global.forceAtlas2 = synchronousLayout;

}(typeof globalThis !== 'undefined' ? globalThis : typeof window !== 'undefined' ? window : this));
