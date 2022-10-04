use egg::{rewrite as rw, *};

use super::{ColumnSet, EGraph, Plan, PlanAnalysis, Rewrite};
use crate::types::DataValue;

#[rustfmt::skip]
fn expr_rules() -> Vec<Rewrite> { vec![
    rw!("add-zero";  "(+ ?a 0)" => "?a"),
    rw!("add-comm";  "(+ ?a ?b)" => "(+ ?b ?a)"),
    rw!("add-assoc"; "(+ ?a (+ ?b ?c))" => "(+ (+ ?a ?b) ?c)"),
    rw!("add-same";  "(+ ?a ?a)" => "(* ?a 2)"),
    rw!("add-neg";   "(+ ?a (- ?b))" => "(- ?a ?b)"),

    rw!("mul-zero";  "(* ?a 0)" => "0"),
    rw!("mul-one";   "(* ?a 1)" => "?a"),
    rw!("mul-minus"; "(* ?a -1)" => "(- ?a)"),
    rw!("mul-comm";  "(* ?a ?b)"        => "(* ?b ?a)"),
    rw!("mul-assoc"; "(* ?a (* ?b ?c))" => "(* (* ?a ?b) ?c)"),

    // rw!("sub-canon"; "(- ?a ?b)" => "(+ ?a (* -1 ?b))"),
    // rw!("canon-sub"; "(+ ?a (* -1 ?b))" => "(- ?a ?b)"),

    rw!("neg-neg";    "(- (- ?a))" => "?a"),
    rw!("neg-sub";    "(- (- ?a ?b))" => "(- ?b ?a)"),

    rw!("sub-zero";   "(- ?a 0)" => "?a"),
    rw!("zero-sub";   "(- 0 ?a)" => "(- ?a)"),
    rw!("sub-cancel"; "(- ?a ?a)" => "0"),

    rw!("div-cancel"; "(/ ?a ?a)" => "1" if is_not_zero("?a")),

    rw!("mul-add-distri";   "(* ?a (+ ?b ?c))" => "(+ (* ?a ?b) (* ?a ?c))"),
    rw!("mul-add-factor";   "(+ (* ?a ?b) (* ?a ?c))" => "(* ?a (+ ?b ?c))"),

    rw!("recip-mul-div"; "(* ?x (/ 1 ?x))" => "1" if is_not_zero("?x")),

    rw!("eq-eq";     "(=  ?a ?a)" => "true"),
    rw!("ne-eq";     "(<> ?a ?a)" => "false"),
    rw!("gt-eq";     "(>  ?a ?a)" => "false"),
    rw!("lt-eq";     "(<  ?a ?a)" => "false"),
    rw!("ge-eq";     "(>= ?a ?a)" => "true"),
    rw!("le-eq";     "(<= ?a ?a)" => "true"),
    rw!("eq-comm";   "(=  ?a ?b)" => "(=  ?b ?a)"),
    rw!("ne-comm";   "(<> ?a ?b)" => "(<> ?b ?a)"),
    rw!("gt-comm";   "(>  ?a ?b)" => "(<  ?b ?a)"),
    rw!("lt-comm";   "(<  ?a ?b)" => "(>  ?b ?a)"),
    rw!("ge-comm";   "(>= ?a ?b)" => "(<= ?b ?a)"),
    rw!("le-comm";   "(<= ?a ?b)" => "(>= ?b ?a)"),
    rw!("eq-add";    "(=  (+ ?a ?b) ?c)" => "(=  ?a (- ?c ?b))"),
    rw!("ne-add";    "(<> (+ ?a ?b) ?c)" => "(<> ?a (- ?c ?b))"),
    rw!("gt-add";    "(>  (+ ?a ?b) ?c)" => "(>  ?a (- ?c ?b))"),
    rw!("lt-add";    "(<  (+ ?a ?b) ?c)" => "(<  ?a (- ?c ?b))"),
    rw!("ge-add";    "(>= (+ ?a ?b) ?c)" => "(>= ?a (- ?c ?b))"),
    rw!("le-add";    "(<= (+ ?a ?b) ?c)" => "(<= ?a (- ?c ?b))"),
    rw!("eq-trans";  "(and (= ?a ?b) (= ?b ?c))" => "(and (= ?a ?b) (= ?a ?c))"),

    rw!("not-eq";    "(not (=  ?a ?b))" => "(<> ?a ?b)"),
    rw!("not-ne";    "(not (<> ?a ?b))" => "(=  ?a ?b)"),
    rw!("not-gt";    "(not (>  ?a ?b))" => "(<= ?a ?b)"),
    rw!("not-ge";    "(not (>= ?a ?b))" => "(<  ?a ?b)"),
    rw!("not-lt";    "(not (<  ?a ?b))" => "(>= ?a ?b)"),
    rw!("not-le";    "(not (<= ?a ?b))" => "(>  ?a ?b)"),
    rw!("not-and";   "(not (and ?a ?b))" => "(or  (not ?a) (not ?b))"),
    rw!("not-or";    "(not (or  ?a ?b))" => "(and (not ?a) (not ?b))"),
    rw!("not-not";   "(not (not ?a))"    => "?a"),

    rw!("and-false"; "(and false ?a)"   => "false"),
    rw!("and-true";  "(and true ?a)"    => "?a"),
    rw!("and-null";  "(and null ?a)"    => "?a"),
    rw!("and-comm";  "(and ?a ?b)"      => "(and ?b ?a)"),
    rw!("and-assoc"; "(and ?a (and ?b ?c))" => "(and (and ?a ?b) ?c)"),

    rw!("or-false";  "(or false ?a)" => "?a"),
    rw!("or-true";   "(or true ?a)"  => "true"),
    rw!("or-null";   "(or null ?a)"  => "?a"),
    rw!("or-comm";   "(or ?a ?b)"    => "(or ?b ?a)"),
    rw!("or-assoc";  "(or ?a (or ?b ?c))" => "(or (or ?a ?b) ?c)"),

    rw!("avg";       "(avg ?a)" => "(/ (sum ?a) (count ?a))"),
]}

#[rustfmt::skip]
fn plan_rules() -> Vec<Rewrite> { vec![
    rw!("limit-order=topn";
        "(limit ?offset ?limit (order ?keys ?child))" =>
        "(topn ?offset ?limit ?keys ?child)"
    ),
    rw!("filter-merge";
        "(filter (filter ?cond1 ?child) ?cond2)" =>
        "(filter (and ?cond1 ?cond2) ?child)"
    ),
    rw!("proj-merge";
        "(proj ?exprs1 (proj ?exprs2 ?child))" =>
        "(proj ?exprs1 ?child)"
    ),
    pushdown("proj", "?exprs", "order", "?keys"),
    pushdown("proj", "?exprs", "limit", "?offset ?limit"),
    pushdown("proj", "?exprs", "topn", "?offset ?limit ?keys"),
    pushdown("filter", "?cond", "order", "?keys"),
    pushdown("filter", "?cond", "limit", "?offset ?limit"),
    pushdown("filter", "?cond", "topn", "?offset ?limit ?keys"),
    rw!("pushdown-filter-join";
        "(filter ?cond (join ?type ?on ?left ?right))" =>
        "(join ?type (and ?on ?cond) ?left ?right)"
    ),
    rw!("pushdown-join-left";
        "(join ?type (and ?cond1 ?cond2) ?left ?right)" =>
        "(join ?type ?cond2 (filter ?cond1 ?left) ?right)"
        if columns_is_subset("?cond1", "?left")
    ),
    rw!("pushdown-join-right";
        "(join ?type (and ?cond1 ?cond2) ?left ?right)" =>
        "(join ?type ?cond2 ?left (filter ?cond1 ?right))"
        if columns_is_subset("?cond1", "?right")
    ),
    rw!("join-reorder";
        "(join inner ?cond2 (join inner ?cond1 ?left ?mid) ?right)" =>
        "(join inner ?cond1 ?left (join inner ?cond2 ?mid ?right))"
        if columns_is_disjoint("?cond2", "?left")
    ),
    rw!("hash-join-on-one-eq";
        "(join ?type (= ?el ?er) ?left ?right)" =>
        "(hashjoin ?type (list ?el) (list ?er) ?left ?right)"
        if columns_is_subset("?el", "?left")
        if columns_is_subset("?er", "?right")
    ),
    rw!("split-projagg";
        "(projagg ?exprs ?groupby ?child)" =>
        { ExtractAgg {
            has_agg: pattern("(proj ?exprs (agg ?aggs ?groupby ?child))"),
            no_agg: pattern("(proj ?exprs ?child)"),
            src: var("?exprs"),
            output: var("?aggs"),
        }}
    ),

    rw!("limit-null";   "(limit null null ?child)" => "?child"),
    rw!("limit-0";      "(limit ?offset 0 ?child)" => "(values)"),
    rw!("filter-true";  "(filter true ?child)" => "?child"),
    rw!("filter-false"; "(filter false ?child)" => "(values)"),
    rw!("join-on-false"; "(join ?type false ?left ?right)" => "(values)"),
    rw!("order-null";   "(order (list) ?child)" => "?child"),

    rw!("select-to-plan";
        "(select ?exprs ?from ?where ?groupby ?having ?orderby ?limit ?offset)" =>
        "
        (limit ?limit ?offset
        (order ?orderby
        (filter ?having
        (projagg ?exprs ?groupby
        (filter ?where
        ?from
        )))))"
    ),
]}

#[rustfmt::skip]
fn column_prune_rules() -> Vec<Rewrite> { vec![
    // projection is the source of prune node
    rw!("prune-gen";
        "(proj ?exprs ?child)" =>
        "(proj ?exprs (prune ?exprs ?child))"
    ),
    // then it is pushed down through the plan node tree,
    // merging all used columns along the way
    rw!("prune-limit";
        "(prune ?set (limit ?offset ?limit ?child))" =>
        "(limit ?offset ?limit (prune ?set ?child))"
    ),
    // note that we use `+` to represent the union of two column sets.
    // in fact, it doesn't matter what operator we use,
    // because the set of a node is calculated by union all its children.
    // see `analyze_columns()`.
    rw!("prune-order";
        "(prune ?set (order ?keys ?child))" =>
        "(order ?keys (prune (+ ?set ?keys) ?child))"
    ),
    rw!("prune-filter";
        "(prune ?set (filter ?cond ?child))" =>
        "(filter ?cond (prune (+ ?set ?cond) ?child))"
    ),
    rw!("prune-agg";
        "(prune ?set (agg ?aggs ?groupby ?child))" =>
        "(agg ?aggs ?groupby (prune (+ (+ ?set ?aggs) ?groupby) ?child))"
    ),
    rw!("prune-join";
        "(prune ?set (join ?type ?on ?left ?right))" =>
        "(join ?type ?on (prune (+ ?set ?on) ?left) (prune (+ ?set ?on) ?right))"
    ),
    // projection and scan is the sink of prune node
    rw!("prune-proj";
        "(prune ?set (proj ?exprs ?child))" =>
        "(proj (prune ?set ?exprs) ?child))"
    ),
    rw!("prune-scan";
        "(prune ?set (scan ?columns))" =>
        "(scan (prune ?set ?columns))"
    ),
    // finally the prune is applied to a list of expressions
    rw!("prune-list";
        "(prune ?set ?list)" =>
        { PruneList {
            set: var("?set"),
            list: var("?list"),
        }}
        if is_list("?list")
    ),
]}

fn all_rules() -> Vec<Rewrite> {
    let mut rules = expr_rules();
    rules.extend(plan_rules());
    rules.extend(column_prune_rules());
    rules
}

/// Make a rule to pushdown `a` through `b`.
fn pushdown(a: &str, a_args: &str, b: &str, b_args: &str) -> Rewrite {
    let name = format!("pushdown-{a}-{b}");
    let searcher = format!("({a} {a_args} ({b} {b_args} ?child))");
    let applier = format!("({b} {b_args} ({a} {a_args} ?child))");
    Rewrite::new(
        name,
        searcher.parse::<Pattern<_>>().unwrap(),
        applier.parse::<Pattern<_>>().unwrap(),
    )
    .unwrap()
}

fn var(s: &str) -> Var {
    s.parse().expect("invalid variable")
}

fn pattern(s: &str) -> Pattern<Plan> {
    s.parse().expect("invalid pattern")
}

fn value_is(v: &str, f: impl Fn(&DataValue) -> bool) -> impl Fn(&mut EGraph, Id, &Subst) -> bool {
    let v = var(v);
    move |egraph, _, subst| {
        if let Some(n) = &egraph[subst[v]].data.val {
            f(n)
        } else {
            false
        }
    }
}

fn is_not_zero(var: &str) -> impl Fn(&mut EGraph, Id, &Subst) -> bool {
    value_is(var, |v| !v.is_zero())
}

fn is_const(var: &str) -> impl Fn(&mut EGraph, Id, &Subst) -> bool {
    value_is(var, |_| true)
}

/// Returns true if the columns in `var1` are a subset of the columns in `var2`.
fn columns_is_subset(var1: &str, var2: &str) -> impl Fn(&mut EGraph, Id, &Subst) -> bool {
    columns_is(var1, var2, ColumnSet::is_subset)
}

/// Returns true if the columns in `var1` has no elements in common with the columns in `var2`.
fn columns_is_disjoint(var1: &str, var2: &str) -> impl Fn(&mut EGraph, Id, &Subst) -> bool {
    columns_is(var1, var2, ColumnSet::is_disjoint)
}

fn columns_is(
    var1: &str,
    var2: &str,
    f: impl Fn(&ColumnSet, &ColumnSet) -> bool,
) -> impl Fn(&mut EGraph, Id, &Subst) -> bool {
    let var1 = var(var1);
    let var2 = var(var2);
    move |egraph, _, subst| {
        let var1_set = &egraph[subst[var1]].data.columns;
        let var2_set = &egraph[subst[var2]].data.columns;
        f(var1_set, var2_set)
    }
}

fn is_list(v: &str) -> impl Fn(&mut EGraph, Id, &Subst) -> bool {
    let v = var(v);
    // we have no rule to rewrite a list,
    // so it should only contains one `Plan::List` in `nodes`.
    move |egraph, _, subst| matches!(egraph[subst[v]].nodes.first(), Some(Plan::List(_)))
}

/// Extracts all agg expressions from `src`.
/// If any, apply `has_agg` and put those aggs to `output`.
/// Otherwise, apply `no_agg`.
struct ExtractAgg {
    has_agg: Pattern<Plan>,
    no_agg: Pattern<Plan>,
    src: Var,
    output: Var,
}

impl Applier<Plan, PlanAnalysis> for ExtractAgg {
    fn apply_one(
        &self,
        egraph: &mut EGraph,
        eclass: Id,
        subst: &Subst,
        searcher_ast: Option<&PatternAst<Plan>>,
        rule_name: Symbol,
    ) -> Vec<Id> {
        let aggs = egraph[subst[self.src]].data.aggs.clone();
        if aggs.is_empty() {
            // FIXME: what if groupby not empty??
            return self
                .no_agg
                .apply_one(egraph, eclass, &subst, searcher_ast, rule_name);
        }
        let mut list: Box<[Id]> = aggs.into_iter().map(|agg| egraph.add(agg)).collect();
        // make sure the order of the aggs is deterministic
        list.sort();
        let mut subst = subst.clone();
        subst.insert(self.output, egraph.add(Plan::List(list)));
        self.has_agg
            .apply_one(egraph, eclass, &subst, searcher_ast, rule_name)
    }
}

/// Remove unused columns in `set` from `list`.
struct PruneList {
    set: Var,
    list: Var,
}

impl Applier<Plan, PlanAnalysis> for PruneList {
    fn apply_one(
        &self,
        egraph: &mut EGraph,
        eclass: Id,
        subst: &Subst,
        _searcher_ast: Option<&PatternAst<Plan>>,
        _rule_name: Symbol,
    ) -> Vec<Id> {
        let used_columns = &egraph[subst[self.set]].data.columns;
        let list = match &egraph[subst[self.list]].nodes[0] {
            Plan::List(list) => list.as_slice(),
            _ => unreachable!("should be a list"),
        };
        let pruned = (list.iter().cloned())
            .filter(|id| !egraph[*id].data.columns.is_disjoint(&used_columns))
            .collect();
        let id = egraph.add(Plan::List(pruned));

        // copied from `Pattern::apply_one`
        if egraph.union(eclass, id) {
            vec![eclass]
        } else {
            vec![]
        }
    }
}

egg::test_fn! {
    and_eq_const,
    expr_rules(),
    "(and (= a 1) (= a b))" => "(and (= a 1) (= b 1))",
}

egg::test_fn! {
    constant_folding,
    expr_rules(),
    "(* (- (+ 1 2) 4) (/ 6 2))" => "-3",
}

egg::test_fn! {
    constant_moving,
    expr_rules(),
    "(> (+ 100 a) 300)" => "(> a 200)",
}

egg::test_fn! {
    predicate_pushdown,
    all_rules(),
    // SELECT s.name, e.cid
    // FROM student AS s, enrolled AS e
    // WHERE s.sid = e.sid AND e.grade = 'A'
    "
    (proj (list $1.2 $2.2)
    (filter (and (= $1.1 $2.1) (= $2.3 'A'))
    (join inner true
        (scan (list $1.1 $1.2))
        (scan (list $2.1 $2.2 $2.3))
    )))" => "
    (proj (list $1.2 $2.2)
    (join inner (= $1.1 $2.1)
        (scan (list $1.1 $1.2))
        (filter (= $2.3 'A')
            (scan (list $2.1 $2.2 $2.3))
        )
    ))"
}

egg::test_fn! {
    join_reorder,
    all_rules(),
    // SELECT * FROM t1, t2, t3
    // WHERE t1.id = t2.id AND t3.id = t2.id
    "
    (filter (and (= $1.1 $2.1) (= $3.1 $2.1))
    (join inner true
        (join inner true
            (scan (list $1.1 $1.2))
            (scan (list $2.1 $2.2))
        )
        (scan (list $3.1 $3.2))
    ))" => "
    (join inner (= $1.1 $2.1)
        (scan (list $1.1 $1.2))
        (join inner (= $2.1 $3.1)
            (scan (list $2.1 $2.2))
            (scan (list $3.1 $3.2))
        )
    )"
}

egg::test_fn! {
    hash_join,
    all_rules(),
    // SELECT * FROM t1, t2
    // WHERE t1.id = t2.id AND t1.age > 2
    "
    (filter (and (= $1.1 $2.1) (> $1.2 2))
    (join inner true
        (scan (list $1.1 $1.2))
        (scan (list $2.1 $2.2))
    ))" => "
    (hashjoin inner (list $1.1) (list $2.1)
        (filter (> $1.2 2)
            (scan (list $1.1 $1.2))
        )
        (scan (list $2.1 $2.2))
    )"
}

egg::test_fn! {
    split_proj_agg,
    all_rules(),
    // SELECT sum(a + b) + count(a) + a FROM t GROUP BY a;
    "
    (projagg
        (list (+ (+ (sum (+ $1.1 $1.2)) (count $1.1)) $1.1))
        (list $1.1)
        (scan (list $1.1 $1.2 $1.3))
    )" => "
    (proj
        (list (+ (+ (sum (+ $1.1 $1.2)) (count $1.1)) $1.1))
        (agg
            (list (sum (+ $1.1 $1.2)) (count $1.1))
            (list $1.1)
            (scan (list $1.1 $1.2 $1.3))
        )
    )"
}

egg::test_fn! {
    no_agg,
    all_rules(),
    // SELECT a FROM t;
    "
    (projagg (list $1.1) (list)
        (scan (list $1.1 $1.2 $1.3))
    )" => "
    (proj (list $1.1)
        (scan (list $1.1 $1.2 $1.3))
    )"
}

egg::test_fn! {
    column_prune,
    all_rules(),
    // SELECT a FROM t1(id, a) JOIN t2(id, b, c) ON t1.id = t2.id WHERE a + b > 1;
    "
    (proj (list $1.2)
    (filter (> (+ $1.2 $2.2) 1)
    (join inner (= $1.1 $2.1)
        (scan (list $1.1 $1.2))
        (scan (list $2.1 $2.2 $2.3))
    )))" => "
    (proj (list $1.2)
    (filter (> (+ $1.2 $2.2) 1)
    (join inner (= $1.1 $2.1)
        (scan (list $1.1 $1.2))
        (scan (list $2.1 $2.2))
    )))"
}

egg::test_fn! {
    plan_select,
    all_rules(),
    // SELECT s.name, e.cid
    // FROM student AS s, enrolled AS e
    // WHERE s.sid = e.sid AND e.grade = 'A'
    "
    (select
        (list $1.2 $2.2)
        (join inner true
            (scan (list $1.1 $1.2))
            (scan (list $2.1 $2.2 $2.3))
        )
        (and (= $1.1 $2.1) (= $2.3 'A'))
        (list)
        true
        (list)
        null
        null
    )" => "
    (proj (list $1.2 $2.2)
    (join inner (= $1.1 $2.1)
        (scan (list $1.1 $1.2))
        (filter (= $2.3 'A')
            (scan (list $2.1 $2.2 $2.3))
        )
    ))"
}

#[test]
fn show_schema() {
    let start = "
    (projagg
        (list (+ (sum (+ $1.1 $1.2)) (count $1.1)))
        (list $1.1)
        (scan (list $1.1 $1.2 $1.3))
    )"
    .parse()
    .unwrap();
    let mut egraph = EGraph::default();
    let id = egraph.add_expr(&start);
    let aggs = egraph[id].data.aggs.clone();
    panic!(
        "{:#?}",
        aggs.iter().map(|plan| plan.to_string()).collect::<Vec<_>>()
    );
}
