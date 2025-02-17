use crate::onto::individual::Individual;
use crate::onto::resource::Resource;
use crate::onto::resource::Value::{Bool, Datetime, Int, Num, Str, Uri};
use chrono::{TimeZone, Utc};
use sqlparser::ast::TableFactor::UNNEST;
use sqlparser::ast::{
    Cte, Expr, Fetch, Function, FunctionArg, FunctionArgExpr, Join, JoinConstraint, JoinOperator, LateralView, ListAgg, ListAggOnOverflow, Offset,
    OrderByExpr, Query, Select, SelectItem, SetExpr, Statement, TableFactor, TableWithJoins, Top, Values, WindowSpec, With,
};
use std::io;
use std::io::{Error, ErrorKind};

pub fn tr_statement(f: &mut Statement, args_map: &mut Individual) -> io::Result<()> {
    if let Statement::Query(ref mut s) = f {
        tr_query(s, args_map)?;
        Ok(())
    } else {
        Err(Error::new(ErrorKind::Unsupported, "Query forbidden".to_string()))
    }
}

fn tr_query(f: &mut Query, args_map: &mut Individual) -> io::Result<()> {
    if let Some(with) = &mut f.with {
        tr_with(with, args_map)?;
    }
    tr_set_expr(&mut f.body, args_map)?;
    if !f.order_by.is_empty() {
        for x in f.order_by.iter_mut() {
            tr_order_by_expr(x, args_map)?;
        }
    }
    if let Some(ref mut limit) = f.limit {
        tr_expr(limit, args_map)?;
    }
    if let Some(ref mut offset) = f.offset {
        tr_offset(offset, args_map)?;
    }
    if let Some(ref mut fetch) = f.fetch {
        tr_fetch(fetch, args_map)?;
    }
    Ok(())
}

fn tr_offset(f: &mut Offset, args_map: &mut Individual) -> io::Result<()> {
    tr_expr(&mut f.value, args_map)?;
    Ok(())
}

fn tr_fetch(f: &mut Fetch, args_map: &mut Individual) -> io::Result<()> {
    if let Some(ref mut quantity) = f.quantity {
        tr_expr(quantity, args_map)?;
    }
    Ok(())
}

fn tr_order_by_expr(f: &mut OrderByExpr, args_map: &mut Individual) -> io::Result<()> {
    tr_expr(&mut f.expr, args_map)?;
    Ok(())
}

fn tr_with(f: &mut With, args_map: &mut Individual) -> io::Result<()> {
    for x in f.cte_tables.iter_mut() {
        tr_cte(x, args_map)?;
    }
    Ok(())
}

fn tr_cte(f: &mut Cte, args_map: &mut Individual) -> io::Result<()> {
    tr_query(&mut f.query, args_map)?;
    Ok(())
}

fn tr_set_expr(f: &mut SetExpr, args_map: &mut Individual) -> io::Result<()> {
    match f {
        SetExpr::Select(s) => {
            tr_select(s, args_map)?;
        },
        SetExpr::Query(q) => {
            tr_query(q, args_map)?;
        },
        SetExpr::Values(v) => {
            tr_values(v, args_map)?;
        },
        SetExpr::Insert(v) => {
            tr_statement(v, args_map)?;
        },
        SetExpr::SetOperation {
            ref mut left,
            ref mut right,
            op: _,
            all: _,
        } => {
            tr_set_expr(left, args_map)?;
            tr_set_expr(right, args_map)?;
        },
    }
    Ok(())
}

fn tr_values(f: &mut Values, args_map: &mut Individual) -> io::Result<()> {
    for row in f.0.iter_mut() {
        for x in row.iter_mut() {
            tr_expr(x, args_map)?;
        }
    }
    Ok(())
}

fn tr_expr(f: &mut Expr, args_map: &mut Individual) -> io::Result<()> {
    match f {
        Expr::MapAccess {
            column,
            keys,
        } => {
            tr_expr(column, args_map)?;
            for k in keys {
                match k {
                    Expr::Value(v) => {
                        if let Some(m) = args_map.obj.resources.get(&v.to_string()) {
                            *v = resource_val_to_sql_val(m.get(0))?;
                        }
                    },
                    _ => {
                        tr_expr(k, args_map)?;
                    },
                }
            }
            return Ok(());
        },
        Expr::InList {
            expr,
            list,
            negated: _,
        } => {
            tr_expr(expr, args_map)?;
            for x in list.iter_mut() {
                tr_expr(x, args_map)?;
            }
        },
        Expr::InSubquery {
            expr,
            subquery,
            negated: _,
        } => {
            tr_expr(expr, args_map)?;
            tr_query(subquery, args_map)?;
        },
        Expr::InUnnest {
            expr,
            array_expr,
            negated: _,
        } => {
            tr_expr(expr, args_map)?;
            tr_expr(array_expr, args_map)?;
        },
        Expr::Between {
            expr,
            negated: _,
            low,
            high,
        } => {
            tr_expr(expr, args_map)?;
            tr_expr(low, args_map)?;
            tr_expr(high, args_map)?;
        },
        Expr::BinaryOp {
            left,
            op: _,
            right,
        } => {
            tr_expr(left, args_map)?;
            tr_expr(right, args_map)?;
        },
        Expr::AnyOp(expr) => {
            tr_expr(expr, args_map)?;
        },
        Expr::AllOp(expr) => {
            tr_expr(expr, args_map)?;
        },
        Expr::UnaryOp {
            op: _,
            expr,
        } => {
            tr_expr(expr, args_map)?;
        },
        Expr::Cast {
            expr,
            data_type: _,
        } => {
            tr_expr(expr, args_map)?;
        },
        Expr::TryCast {
            expr,
            data_type: _,
        } => {
            tr_expr(expr, args_map)?;
        },
        Expr::Extract {
            field: _,
            expr,
        } => {
            tr_expr(expr, args_map)?;
        },
        Expr::Position {
            expr,
            r#in,
        } => {
            tr_expr(expr, args_map)?;
            tr_expr(r#in, args_map)?;
        },
        Expr::Collate {
            expr,
            collation: _,
        } => {
            tr_expr(expr, args_map)?;
        },
        Expr::Nested(ast) => {
            tr_expr(ast, args_map)?;
        },
        Expr::Value(v) => {
            let v_s = v.to_string();
            if v_s.starts_with("'{") && v_s.ends_with("}'") {
                let val = v_s[2..v_s.len() - 2].to_string();
                if let Some(m) = args_map.obj.resources.get(&val) {
                    *v = resource_val_to_sql_val(m.get(0))?;
                }
            }
        },
        Expr::Function(ref mut fun) => {
            tr_function(fun, args_map)?;
        },
        Expr::Case {
            operand,
            conditions,
            results,
            else_result,
        } => {
            if let Some(operand) = operand {
                tr_expr(operand, args_map)?;
            }
            for (c, r) in conditions.iter_mut().zip(results) {
                tr_expr(c, args_map)?;
                tr_expr(r, args_map)?;
            }

            if let Some(else_result) = else_result {
                tr_expr(else_result, args_map)?;
            }
        },
        //Expr::Exists(s) => {
        //    tr_query(s, args_map)?;
        //},
        Expr::Subquery(s) => {
            tr_query(s, args_map)?;
        },
        Expr::ListAgg(listagg) => {
            tr_list_agg(listagg, args_map)?;
        },
        Expr::GroupingSets(sets) => {
            for set in sets {
                for x in set.iter_mut() {
                    tr_expr(x, args_map)?;
                }
            }
        },
        Expr::Cube(sets) => {
            for set in sets {
                for x in set.iter_mut() {
                    tr_expr(x, args_map)?;
                }
            }
        },
        Expr::Rollup(ref mut sets) => {
            for set in sets.iter_mut() {
                if set.len() == 1 {
                    tr_expr(&mut set[0], args_map)?;
                } else {
                    for x in set.iter_mut() {
                        tr_expr(x, args_map)?;
                    }
                }
            }
        },
        Expr::Substring {
            expr,
            substring_from,
            substring_for,
        } => {
            tr_expr(expr, args_map)?;
            if let Some(ref mut from_part) = substring_from {
                tr_expr(from_part, args_map)?;
            }
            if let Some(ref mut from_part) = substring_for {
                tr_expr(from_part, args_map)?;
            }
        },
        Expr::IsDistinctFrom(ref mut a, ref mut b) => {
            tr_expr(a, args_map)?;
            tr_expr(b, args_map)?;
        },
        Expr::IsNotDistinctFrom(ref mut a, ref mut b) => {
            tr_expr(a, args_map)?;
            tr_expr(b, args_map)?;
        },
        Expr::Trim {
            ref mut expr,
            trim_where: _,
            trim_what: _,
        } => {
            tr_expr(expr, args_map)?;
        },
        Expr::Tuple(exprs) => {
            for x in exprs.iter_mut() {
                tr_expr(x, args_map)?;
            }
        },
        Expr::ArrayIndex {
            ref mut obj,
            indexes,
        } => {
            tr_expr(obj, args_map)?;

            for i in indexes.iter_mut() {
                tr_expr(i, args_map)?;
            }
            return Ok(());
        },
        Expr::Array(ref mut set) => {
            for x in set.elem.iter_mut() {
                tr_expr(x, args_map)?;
            }
        },
        Expr::JsonAccess {
            ref mut left,
            operator: _,
            ref mut right,
        } => {
            tr_expr(left, args_map)?;
            tr_expr(right, args_map)?;
        },
        Expr::CompositeAccess {
            ref mut expr,
            key: _,
        } => {
            tr_expr(expr, args_map)?;
        },
        _ => {},
    }
    Ok(())
}

fn tr_list_agg(f: &mut ListAgg, args_map: &mut Individual) -> io::Result<()> {
    tr_expr(&mut f.expr, args_map)?;

    if let Some(ref mut separator) = f.separator {
        tr_expr(separator, args_map)?;
    }
    if let Some(ref mut on_overflow) = f.on_overflow {
        tr_list_agg_on_overflow(on_overflow, args_map)?;
    }
    if !f.within_group.is_empty() {
        for x in f.within_group.iter_mut() {
            tr_order_by_expr(x, args_map)?;
        }
    }
    Ok(())
}

fn tr_list_agg_on_overflow(f: &mut ListAggOnOverflow, args_map: &mut Individual) -> io::Result<()> {
    if let ListAggOnOverflow::Truncate {
        filler: Some(filler),
        with_count: _,
    } = f
    {
        tr_expr(filler, args_map)?;
    }

    Ok(())
}

fn tr_select_item(f: &mut SelectItem, args_map: &mut Individual) -> io::Result<()> {
    match f {
        SelectItem::UnnamedExpr(ref mut expr) => {
            tr_expr(expr, args_map)?;
        },
        SelectItem::ExprWithAlias {
            ref mut expr,
            alias: _,
        } => {
            tr_expr(expr, args_map)?;
        },
        _ => {},
    }
    Ok(())
}

fn tr_select(f: &mut Select, args_map: &mut Individual) -> io::Result<()> {
    if let Some(ref mut top) = f.top {
        tr_top(top, args_map)?;
    }
    for x in f.projection.iter_mut() {
        tr_select_item(x, args_map)?;
    }

    if !f.from.is_empty() {
        for x in f.from.iter_mut() {
            tr_table_with_joins(x, args_map)?;
        }
    }
    if !f.lateral_views.is_empty() {
        for lv in f.lateral_views.iter_mut() {
            tr_lateral_view(lv, args_map)?;
        }
    }
    if let Some(ref mut selection) = f.selection {
        tr_expr(selection, args_map)?;
    }
    if !f.group_by.is_empty() {
        for x in f.group_by.iter_mut() {
            tr_expr(x, args_map)?;
        }
    }
    if !f.cluster_by.is_empty() {
        for x in f.cluster_by.iter_mut() {
            tr_expr(x, args_map)?;
        }
    }
    if !f.distribute_by.is_empty() {
        for x in f.distribute_by.iter_mut() {
            tr_expr(x, args_map)?;
        }
    }
    if !f.sort_by.is_empty() {
        for x in f.sort_by.iter_mut() {
            tr_expr(x, args_map)?;
        }
    }
    if let Some(ref mut having) = f.having {
        tr_expr(having, args_map)?;
    }
    if let Some(ref mut qualify) = f.qualify {
        tr_expr(qualify, args_map)?;
    }
    Ok(())
}

fn tr_top(f: &mut Top, args_map: &mut Individual) -> io::Result<()> {
    if let Some(ref mut quantity) = f.quantity {
        tr_expr(quantity, args_map)?;
    }
    Ok(())
}

fn tr_table_with_joins(f: &mut TableWithJoins, args_map: &mut Individual) -> io::Result<()> {
    tr_table_factor(&mut f.relation, args_map)?;
    for join in f.joins.iter_mut() {
        tr_join(join, args_map)?;
    }
    Ok(())
}

fn tr_join_constraint(f: &mut JoinConstraint, args_map: &mut Individual) -> io::Result<()> {
    if let JoinConstraint::On(ref mut expr) = f {
        tr_expr(expr, args_map)?;
    }
    Ok(())
}

fn tr_join(f: &mut Join, args_map: &mut Individual) -> io::Result<()> {
    match &mut f.join_operator {
        JoinOperator::Inner(ref mut constraint) => {
            tr_table_factor(&mut f.relation, args_map)?;
            tr_join_constraint(constraint, args_map)?;
        },
        JoinOperator::LeftOuter(constraint) => {
            tr_table_factor(&mut f.relation, args_map)?;
            tr_join_constraint(constraint, args_map)?;
        },
        JoinOperator::RightOuter(constraint) => {
            tr_table_factor(&mut f.relation, args_map)?;
            tr_join_constraint(constraint, args_map)?;
        },
        JoinOperator::FullOuter(constraint) => {
            tr_join_constraint(constraint, args_map)?;
            tr_table_factor(&mut f.relation, args_map)?;
        },
        JoinOperator::CrossJoin => {
            tr_table_factor(&mut f.relation, args_map)?;
        },
        JoinOperator::CrossApply => {
            tr_table_factor(&mut f.relation, args_map)?;
        },
        JoinOperator::OuterApply => {
            tr_table_factor(&mut f.relation, args_map)?;
        },
    }
    Ok(())
}

fn tr_table_factor(f: &mut TableFactor, args_map: &mut Individual) -> io::Result<()> {
    match f {
        &mut UNNEST {
            ..
        } => todo!(),
        TableFactor::Table {
            name,
            alias: _,
            args,
            with_hints,
        } => {
            if name.to_string().as_str() == "url" {
                return Err(Error::new(ErrorKind::Unsupported, format!("Table [{}] forbidden", name)));
            }

            if let Some(a) = args {
                for x in a.iter_mut() {
                    tr_function_arg(x, args_map)?;
                }
            }

            if !with_hints.is_empty() {
                for x in with_hints.iter_mut() {
                    tr_expr(x, args_map)?;
                }
            }
        },
        TableFactor::Derived {
            lateral: _,
            subquery,
            alias: _,
        } => {
            tr_query(subquery, args_map)?;
        },
        TableFactor::TableFunction {
            expr,
            alias: _,
        } => {
            tr_expr(expr, args_map)?;
        },
        TableFactor::NestedJoin {
            table_with_joins,
            alias: _,
        } => {
            tr_table_with_joins(table_with_joins, args_map)?;
        },
    }
    Ok(())
}

fn tr_function_arg(f: &mut FunctionArg, args_map: &mut Individual) -> io::Result<()> {
    match f {
        FunctionArg::Named {
            name: _,
            arg,
        } => {
            tr_function_arg_expr(arg, args_map)?;
        },
        FunctionArg::Unnamed(unnamed_arg) => {
            tr_function_arg_expr(unnamed_arg, args_map)?;
        },
    }
    Ok(())
}

fn tr_function_arg_expr(f: &mut FunctionArgExpr, args_map: &mut Individual) -> io::Result<()> {
    if let FunctionArgExpr::Expr(expr) = f {
        tr_expr(expr, args_map)?;
    }
    Ok(())
}

fn tr_function(f: &mut Function, args_map: &mut Individual) -> io::Result<()> {
    match f.name.to_string().as_str() {
        "sleep" | "url" => {
            return Err(Error::new(ErrorKind::Unsupported, format!("Function [{}] forbidden", f.name)));
        },
        _ => {},
    }

    for x in f.args.iter_mut() {
        tr_function_arg(x, args_map)?;
    }

    if let Some(ref mut o) = f.over {
        tr_window_spec(o, args_map)?;
    }
    Ok(())
}

fn tr_window_spec(f: &mut WindowSpec, args_map: &mut Individual) -> io::Result<()> {
    if !f.partition_by.is_empty() {
        for x in f.partition_by.iter_mut() {
            tr_expr(x, args_map)?;
        }
    }
    if !f.order_by.is_empty() {
        for x in f.order_by.iter_mut() {
            tr_order_by_expr(x, args_map)?;
        }
    }

    Ok(())
}

fn tr_lateral_view(f: &mut LateralView, args_map: &mut Individual) -> io::Result<()> {
    tr_expr(&mut f.lateral_view, args_map)?;
    Ok(())
}

fn resource_val_to_sql_val(ri: Option<&Resource>) -> io::Result<sqlparser::ast::Value> {
    if let Some(r) = ri {
        return match &r.value {
            Uri(v) | Str(v, _) => Ok(sqlparser::ast::Value::SingleQuotedString(v.to_owned())),
            Int(v) => Ok(sqlparser::ast::Value::Number(v.to_string(), false)),
            Bool(v) => Ok(sqlparser::ast::Value::Boolean(*v)),
            Num(_m, _d) => Ok(sqlparser::ast::Value::Number(r.get_float().to_string(), false)),
            Datetime(v) => {
                match Utc.timestamp_opt(*v, 0) {
                    chrono::LocalResult::Single(datetime) => Ok(sqlparser::ast::Value::SingleQuotedString(format!("{:?}", datetime))),
                    // Handle other cases (None, Ambiguous) as needed
                    _ => Err(Error::new(ErrorKind::Other, "Invalid or ambiguous datetime")),
                }
            },
            _ => Err(Error::new(ErrorKind::Other, format!("Unknown type {:?}", r.value))),
        };
    }
    Ok(sqlparser::ast::Value::Null)
}
