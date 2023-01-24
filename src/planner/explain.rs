use std::borrow::Cow;
use std::collections::BTreeMap;
use std::fmt::{Display, Formatter, Result};

use egg::Id;
use maplit::btreemap;
use pretty_xmlish::{Pretty, XmlNode};

use super::{Expr, RecExpr};
use crate::catalog::RootCatalog;

fn pretty_node<'a>(name: impl Into<Cow<'a, str>>, v: Vec<Pretty<'a>>) -> Pretty<'a> {
    named_record(name, Default::default(), v)
}

fn named_record<'a>(
    name: impl Into<Cow<'a, str>>,
    fields: BTreeMap<&'a str, Pretty<'a>>,
    children: Vec<Pretty<'a>>,
) -> Pretty<'a> {
    let fields = fields.into_iter().map(|(k, v)| (k.into(), v)).collect();
    Pretty::Record(XmlNode::new(name.into(), fields, children))
}

/// A wrapper over [`RecExpr`] to explain it in [`Display`].
///
/// # Example
/// ```
/// use risinglight::planner::{Explain, RecExpr};
/// let expr: RecExpr = "(+ 1 2)".parse().unwrap();
/// println!("{}", Explain::of(&expr));
/// ```
pub struct Explain<'a> {
    expr: &'a RecExpr,
    costs: Option<&'a [f32]>,
    catalog: Option<&'a RootCatalog>,
    id: Id,
}

impl<'a> Explain<'a> {
    /// Create a [`Explain`] over [`RecExpr`].
    pub fn of(expr: &'a RecExpr) -> Self {
        Self {
            expr,
            costs: None,
            catalog: None,
            id: Id::from(expr.as_ref().len() - 1),
        }
    }

    /// Explain with costs.
    pub fn with_costs(mut self, costs: &'a [f32]) -> Self {
        self.costs = Some(costs);
        self
    }

    /// Explain column in name.
    pub fn with_catalog(mut self, catalog: &'a RootCatalog) -> Self {
        self.catalog = Some(catalog);
        self
    }

    /// Returns a explain for the sub expression.
    #[inline]
    const fn expr(&self, id: &Id) -> Self {
        Explain {
            expr: self.expr,
            costs: self.costs,
            catalog: self.catalog,
            id: *id,
        }
    }

    /// Returns a explain for the child plan.
    #[inline]
    const fn child(&self, id: &Id) -> Self {
        Explain {
            expr: self.expr,
            costs: self.costs,
            catalog: self.catalog,
            id: *id,
        }
    }

    /// Returns a struct displaying the cost.
    #[inline]
    fn cost(&self) -> Option<f32> {
        self.costs.map(|cs| cs[usize::from(self.id)])
    }

    /// Returns whether the expression is `true`.
    #[inline]
    fn is_true(&self, id: &Id) -> bool {
        self.expr[*id] == Expr::true_()
    }

    pub fn pretty(&self) -> Pretty<'a> {
        use Expr::*;
        let enode = &self.expr[self.id];
        let cost = self.cost();
        match enode {
            Constant(v) => Pretty::display(v),
            Type(t) => Pretty::display(t),
            Table(i) => {
                if let Some(catalog) = self.catalog {
                    catalog.get_table(i).expect("no table").name().into()
                } else {
                    Pretty::display(i)
                }
            }
            Column(i) => {
                if let Some(catalog) = self.catalog {
                    let column_catalog = catalog.get_column(i).expect("no column");
                    column_catalog.into_name().into()
                } else {
                    Pretty::display(i)
                }
            }
            ColumnIndex(i) => Pretty::display(i),

            // TODO: use object
            ExtSource(src) => format!("path={:?}, format={}", src.path, src.format).into(),
            Symbol(s) => Pretty::display(s),
            Ref(e) => self.expr(e).pretty(),
            List(list) => Pretty::Array(list.iter().map(|e| self.expr(e).pretty()).collect()),

            // binary operations
            Add([a, b]) | Sub([a, b]) | Mul([a, b]) | Div([a, b]) | Mod([a, b])
            | StringConcat([a, b]) | Gt([a, b]) | Lt([a, b]) | GtEq([a, b]) | LtEq([a, b])
            | Eq([a, b]) | NotEq([a, b]) | And([a, b]) | Or([a, b]) | Xor([a, b])
            | Like([a, b]) => named_record(
                enode.to_string(),
                btreemap! {
                    "lhs" => self.expr(a).pretty(),
                    "rhs" => self.expr(b).pretty(),
                },
                vec![],
            ),

            // unary operations
            Neg(a) | Not(a) | IsNull(a) => {
                pretty_node(enode.to_string(), vec![self.expr(a).pretty()])
            }

            If([cond, then, else_]) => named_record(
                "If",
                btreemap! {
                    "cond" => self.expr(cond).pretty(),
                    "then" => self.expr(then).pretty(),
                    "else" => self.expr(else_).pretty(),
                },
                vec![],
            ),

            // functions
            Extract([field, e]) => named_record(
                "Extract",
                btreemap! {
                    "from" => self.expr(e).pretty(),
                    "field" => self.expr(field).pretty(),
                },
                vec![],
            ),
            Field(field) => Pretty::display(field),

            // aggregations
            RowCount => "rowcount".into(),
            Max(a) | Min(a) | Sum(a) | Avg(a) | Count(a) | First(a) | Last(a) => {
                pretty_node(enode.to_string(), vec![self.expr(a).pretty()])
            }

            Exists(a) => pretty_node("Exists", vec![self.expr(a).pretty()]),
            In([a, b]) => named_record(
                "In",
                btreemap! {
                    "in" => self.expr(b).pretty(),
                },
                vec![self.expr(a).pretty()],
            ),
            Cast([a, b]) => named_record(
                "Cast",
                btreemap! {
                    "type" => self.expr(b).pretty(),
                },
                vec![self.expr(a).pretty()],
            ),

            Scan([table, list]) | Internal([table, list]) => named_record(
                "Scan",
                with_cost(
                    cost,
                    btreemap! {
                       "table" => self.expr(table).pretty(),
                       "list" => self.expr(list).pretty()
                    },
                ),
                vec![],
            ),
            Values(rows) => named_record(
                "Values",
                with_cost(
                    cost,
                    btreemap! {
                        "rows" => Pretty::display(&rows.len()),
                    },
                ),
                vec![],
            ),
            Proj([exprs, child]) => named_record(
                "Projection",
                with_cost(
                    cost,
                    btreemap! {
                        "exprs" => self.expr(exprs).pretty(),
                    },
                ),
                vec![self.child(child).pretty()],
            ),
            Filter([cond, child]) => named_record(
                "Filter",
                with_cost(
                    cost,
                    btreemap! {
                        "cond" => self.expr(cond).pretty(),
                    },
                ),
                vec![self.child(child).pretty()],
            ),
            Order([orderby, child]) => named_record(
                "Order",
                with_cost(
                    cost,
                    btreemap! {
                        "by" => self.expr(orderby).pretty(),
                    },
                ),
                vec![self.child(child).pretty()],
            ),
            Asc(a) | Desc(a) => pretty_node(enode.to_string(), vec![self.expr(a).pretty()]),
            Limit([limit, offset, child]) => named_record(
                "Limit",
                with_cost(
                    cost,
                    btreemap! {
                        "limit" => self.expr(limit).pretty(),
                        "offset" => self.expr(offset).pretty(),
                    },
                ),
                vec![self.child(child).pretty()],
            ),
            TopN([limit, offset, orderby, child]) => named_record(
                "TopN",
                with_cost(
                    cost,
                    btreemap! {
                        "limit" => self.expr(limit).pretty(),
                        "offset" => self.expr(offset).pretty(),
                        "order_by" => self.expr(orderby).pretty(),
                    },
                ),
                vec![self.child(child).pretty()],
            ),
            Join([ty, cond, left, right]) => {
                let mut fields = with_cost(
                    cost,
                    btreemap! {
                        "type" => self.expr(ty).pretty(),
                    },
                );

                if !self.is_true(cond) {
                    fields.entry("on").or_insert(self.expr(cond).pretty());
                }
                named_record(
                    "Join",
                    fields,
                    vec![self.child(left).pretty(), self.child(right).pretty()],
                )
            }
            HashJoin([ty, lkeys, rkeys, left, right]) => named_record(
                "HashJoin",
                with_cost(
                    cost,
                    btreemap! {
                        "type" => self.expr(ty).pretty(),
                        "on" => named_record(
                            "Equality",
                            btreemap! {
                                "lhs" => self.expr(lkeys).pretty(),
                                "rhs" => self.expr(rkeys).pretty(),
                            },
                            vec![],
                        ),
                    },
                ),
                vec![self.child(left).pretty(), self.child(right).pretty()],
            ),
            Inner | LeftOuter | RightOuter | FullOuter => Pretty::display(enode),
            Agg([aggs, group_keys, child]) => named_record(
                "Aggregate",
                with_cost(
                    cost,
                    btreemap! {
                        "aggs" => self.expr(aggs).pretty(),
                        "group_by" => self.expr(group_keys).pretty(),
                    },
                ),
                vec![self.child(child).pretty()],
            ),
            CreateTable(t) => {
                let mut fields = with_cost(
                    cost,
                    btreemap! {
                        "name" => Pretty::display(&t.table_name),
                    },
                );
                // TODO
                named_record("CreateTable", fields, vec![])
            }
            Drop(t) => {
                let mut fields = with_cost(
                    cost,
                    btreemap! {
                        "name" => Pretty::display(&t.object),
                    },
                );
                // TODO
                named_record("Drop", fields, vec![])
            }
            _ => todo!(),
            // Insert([table, cols, child]) => write!(
            //     f,
            //     "{tab}Insert: {}{}{cost}\n{}",
            //     self.expr(table),
            //     self.expr(cols),
            //     self.child(child)
            // ),
            // Delete([table, child]) => write!(
            //     f,
            //     "{tab}Delete: from={}{cost}\n{}",
            //     self.expr(table),
            //     self.child(child)
            // ),
            // CopyFrom([src, _]) => writeln!(f, "{tab}CopyFrom: {}{cost}", self.expr(src)),
            // CopyTo([dst, child]) => write!(
            //     f,
            //     "{tab}CopyTo: {}{cost}\n{}",
            //     self.expr(dst),
            //     self.child(child)
            // ),
            // Explain(child) => write!(f, "{tab}Explain:{cost}\n{}", self.child(child)),
            // Empty(_) => writeln!(f, "{tab}Empty:{cost}"),
        }
    }
}

fn with_cost<'a>(
    cost: Option<f32>,
    mut map: BTreeMap<&'a str, Pretty<'a>>,
) -> BTreeMap<&'a str, Pretty<'a>> {
    if let Some(cost) = cost {
        map.entry("cost").or_insert(Pretty::display(&cost));
    }
    map
}
