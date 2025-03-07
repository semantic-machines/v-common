use v_individual_model::onto::individual::Individual;
use v_individual_model::onto::resource::Resource;
use v_individual_model::onto::resource::Value::{Bool, Datetime, Int, Num, Str, Uri};
use crate::search::common::{get_full_prefix, split_short_prefix, PrefixesCache};
use chrono::{TimeZone, Utc};
use oxrdf::vocab::xsd;
use oxrdf::NamedNode;
use spargebra::algebra::{Expression, GraphPattern, OrderExpression};
use spargebra::term::{GroundTerm, Literal, NamedNodePattern, TermPattern, TriplePattern};
use spargebra::Query;
use std::io;
use std::io::{Error, ErrorKind};
use std::str::FromStr;

pub fn prepare_sparql_params(query: &str, params: &mut Individual, prefix_cache: &PrefixesCache) -> Result<String, Error> {
    match Query::parse(query, None) {
        Ok(ref mut sparql) => {
            debug!("{:?}", query);
            if let Query::Select {
                dataset: _,
                ref mut pattern,
                base_iri: _,
            } = sparql
            {
                tr_graph_pattern(pattern, params, prefix_cache)?;
            }

            //                            warn!("{}", sparql);
            return Ok(sparql.to_string());
        },
        Err(e) => {
            error!("{}", e);
        },
    }
    Err(Error::new(ErrorKind::Other, "?"))
}

fn tr_graph_pattern(f: &mut GraphPattern, args_map: &mut Individual, prefix_cache: &PrefixesCache) -> io::Result<()> {
    match f {
        GraphPattern::Bgp {
            patterns,
        } => {
            for el in patterns.iter_mut() {
                tr_triple_pattern(el, args_map, prefix_cache)?;
            }
        },
        GraphPattern::Path {
            path: _,
            subject,
            object,
        } => {
            tr_term_pattern(subject, args_map, prefix_cache)?;
            tr_term_pattern(object, args_map, prefix_cache)?;
        },
        GraphPattern::Join {
            left,
            right,
        } => {
            tr_graph_pattern(left, args_map, prefix_cache)?;
            tr_graph_pattern(right, args_map, prefix_cache)?;
        },
        GraphPattern::LeftJoin {
            left,
            right,
            expression,
        } => {
            tr_graph_pattern(left, args_map, prefix_cache)?;
            tr_graph_pattern(right, args_map, prefix_cache)?;
            if let Some(expr) = expression {
                tr_expression(expr, args_map, prefix_cache)?;
            }
        },
        GraphPattern::Filter {
            expr,
            inner,
        } => {
            tr_graph_pattern(inner, args_map, prefix_cache)?;
            tr_expression(expr, args_map, prefix_cache)?;
        },
        GraphPattern::Union {
            left,
            right,
        } => {
            tr_graph_pattern(left, args_map, prefix_cache)?;
            tr_graph_pattern(right, args_map, prefix_cache)?;
        },
        GraphPattern::Graph {
            ..
        } => {},
        GraphPattern::Extend {
            inner,
            variable: _,
            expression,
        } => {
            tr_graph_pattern(inner, args_map, prefix_cache)?;
            tr_expression(expression, args_map, prefix_cache)?;
        },
        GraphPattern::Minus {
            left,
            right,
        } => {
            tr_graph_pattern(left, args_map, prefix_cache)?;
            tr_graph_pattern(right, args_map, prefix_cache)?;
        },
        GraphPattern::Values {
            variables: _,
            bindings,
        } => {
            for el in bindings.iter_mut() {
                for el1 in el.iter_mut().flatten() {
                    tr_ground_term(el1, args_map, prefix_cache)?;
                }
            }
        },
        GraphPattern::OrderBy {
            inner,
            expression,
        } => {
            tr_graph_pattern(inner, args_map, prefix_cache)?;
            for el in expression.iter_mut() {
                tr_order_expression(el, args_map, prefix_cache)?;
            }
        },
        GraphPattern::Project {
            inner,
            variables,
        } => {
            tr_graph_pattern(inner, args_map, prefix_cache)?;

            for el in variables.iter_mut() {
                debug!("VARIABLE: {}", el);
            }
        },
        GraphPattern::Distinct {
            inner,
        } => {
            tr_graph_pattern(inner, args_map, prefix_cache)?;
        },
        GraphPattern::Reduced {
            inner,
        } => {
            tr_graph_pattern(inner, args_map, prefix_cache)?;
        },
        GraphPattern::Slice {
            inner,
            start: _,
            length: _,
        } => {
            tr_graph_pattern(inner, args_map, prefix_cache)?;
        },
        GraphPattern::Group {
            inner,
            variables: _,
            aggregates: _,
        } => {
            tr_graph_pattern(inner, args_map, prefix_cache)?;
        },
        GraphPattern::Service {
            inner,
            name: _,
            silent: _,
        } => {
            tr_graph_pattern(inner, args_map, prefix_cache)?;
        },
    }
    Ok(())
}

fn tr_order_expression(f: &mut OrderExpression, args_map: &mut Individual, prefix_cache: &PrefixesCache) -> io::Result<()> {
    match f {
        OrderExpression::Asc(expr) => {
            tr_expression(expr, args_map, prefix_cache)?;
        },
        OrderExpression::Desc(expr) => {
            tr_expression(expr, args_map, prefix_cache)?;
        },
    }
    Ok(())
}

fn tr_ground_term(f: &mut GroundTerm, _args_map: &mut Individual, _prefix_cache: &PrefixesCache) -> io::Result<()> {
    match f {
        GroundTerm::NamedNode(_) => {},
        GroundTerm::Literal(v) => {
            debug!("ground_term::LITERAL: {}", v.value());
            //tr_literal(l, args_map, prefix_cache)?;
        },
    }
    Ok(())
}

fn tr_term_pattern(f: &mut TermPattern, _args_map: &mut Individual, _prefix_cache: &PrefixesCache) -> io::Result<()> {
    match f {
        TermPattern::NamedNode(_) => {},
        TermPattern::BlankNode(_) => {},
        TermPattern::Literal(v) => {
            debug!("term_pattern::LITERAL: {}", v.value());
            //tr_literal(v, args_map, prefix_cache)?;
        },
        TermPattern::Variable(_) => {},
    }
    Ok(())
}

fn tr_triple_pattern(f: &mut TriplePattern, args_map: &mut Individual, prefix_cache: &PrefixesCache) -> io::Result<()> {
    match f.subject {
        TermPattern::NamedNode(_) => {},
        TermPattern::BlankNode(_) => {},
        TermPattern::Literal(ref mut v) => {
            //tr_literal(v, args_map, prefix_cache)?;
            debug!("triple_pattern::subject::LITERAL: {}", v.value());
        },
        TermPattern::Variable(_) => {},
    }

    match f.predicate {
        NamedNodePattern::NamedNode(_) => {},
        NamedNodePattern::Variable(_) => {},
    }

    match f.object {
        TermPattern::NamedNode(_) => {},
        TermPattern::BlankNode(_) => {},
        TermPattern::Literal(ref mut v) => {
            //tr_literal(v, args_map, prefix_cache)?;
            debug!("triple_pattern::object::LITERAL: {}", v.value());
            let v_s = v.value();
            if v_s.starts_with('{') && v_s.ends_with('}') {
                let val = v_s[1..v_s.len() - 1].to_string();
                if let Some(m) = args_map.get_obj().get_resources().get(&val) {
                    f.object = resource_val_to_sparql_val(m.get(0), prefix_cache)?;
                }
            }
        },
        TermPattern::Variable(_) => {},
    }

    Ok(())
}
/*
fn tr_literal0(f: &mut spargebra::term::Literal, args_map: &mut Individual, prefix_cache: &PrefixesCache) -> io::Result<()> {
    debug!("LITERAL: {}", f.value());
    if let Some(m) = args_map.obj.resources.get(f.value()) {
        *f = resource_val_to_sparql_val(m.get(0))?;
    }

    Ok(())
}
*/
fn tr_expression(f: &mut Expression, args_map: &mut Individual, prefix_cache: &PrefixesCache) -> io::Result<()> {
    match f {
        Expression::NamedNode(_) => {},
        Expression::Literal(v) => {
            debug!("expression::object::LITERAL: {}", v.value());
            let v_s = v.value();
            if v_s.starts_with('{') && v_s.ends_with('}') {
                let val = v_s[1..v_s.len() - 1].to_string();

                if let Some(m) = args_map.get_obj().get_resources().get(&val) {
                    *f = part_copy_termpattern_to_expression(resource_val_to_sparql_val(m.get(0), prefix_cache)?)?;
                }
            }
            //tr_literal(l, args_map, prefix_cache)?;
        },
        Expression::Variable(_) => {},
        Expression::Or(a, b) => {
            tr_expression(a, args_map, prefix_cache)?;
            tr_expression(b, args_map, prefix_cache)?;
        },
        Expression::And(a, b) => {
            tr_expression(a, args_map, prefix_cache)?;
            tr_expression(b, args_map, prefix_cache)?;
        },
        Expression::Equal(a, b) => {
            tr_expression(a, args_map, prefix_cache)?;
            tr_expression(b, args_map, prefix_cache)?;
        },
        Expression::SameTerm(a, b) => {
            tr_expression(a, args_map, prefix_cache)?;
            tr_expression(b, args_map, prefix_cache)?;
        },
        Expression::Greater(a, b) => {
            tr_expression(a, args_map, prefix_cache)?;
            tr_expression(b, args_map, prefix_cache)?;
        },
        Expression::GreaterOrEqual(a, b) => {
            tr_expression(a, args_map, prefix_cache)?;
            tr_expression(b, args_map, prefix_cache)?;
        },
        Expression::Less(a, b) => {
            tr_expression(a, args_map, prefix_cache)?;
            tr_expression(b, args_map, prefix_cache)?;
        },
        Expression::LessOrEqual(a, b) => {
            tr_expression(a, args_map, prefix_cache)?;
            tr_expression(b, args_map, prefix_cache)?;
        },
        Expression::In(a, b) => {
            tr_expression(a, args_map, prefix_cache)?;

            for el in b.iter_mut() {
                tr_expression(el, args_map, prefix_cache)?;
            }
        },
        Expression::Add(a, b) => {
            tr_expression(a, args_map, prefix_cache)?;
            tr_expression(b, args_map, prefix_cache)?;
        },
        Expression::Subtract(a, b) => {
            tr_expression(a, args_map, prefix_cache)?;
            tr_expression(b, args_map, prefix_cache)?;
        },
        Expression::Multiply(a, b) => {
            tr_expression(a, args_map, prefix_cache)?;
            tr_expression(b, args_map, prefix_cache)?;
        },
        Expression::Divide(a, b) => {
            tr_expression(a, args_map, prefix_cache)?;
            tr_expression(b, args_map, prefix_cache)?;
        },
        Expression::UnaryPlus(a) => {
            tr_expression(a, args_map, prefix_cache)?;
        },
        Expression::UnaryMinus(a) => {
            tr_expression(a, args_map, prefix_cache)?;
        },
        Expression::Not(a) => {
            tr_expression(a, args_map, prefix_cache)?;
        },
        Expression::Exists(a) => {
            tr_graph_pattern(a, args_map, prefix_cache)?;
        },
        Expression::Bound(_) => {},
        Expression::If(a, b, c) => {
            tr_expression(a, args_map, prefix_cache)?;
            tr_expression(b, args_map, prefix_cache)?;
            tr_expression(c, args_map, prefix_cache)?;
        },
        Expression::Coalesce(a) => {
            for el in a.iter_mut() {
                tr_expression(el, args_map, prefix_cache)?;
            }
        },
        Expression::FunctionCall(_f, fc) => {
            for el in fc.iter_mut() {
                tr_expression(el, args_map, prefix_cache)?;
            }
        },
    }
    Ok(())
}

fn resource_val_to_sparql_val(ri: Option<&Resource>, prefix_cache: &PrefixesCache) -> io::Result<TermPattern> {
    if let Some(r) = ri {
        return match &r.value {
            Uri(v) => {
                let iri = if let Some((short_prefix, id)) = split_short_prefix(v) {
                    format!("<{}{}>", get_full_prefix(short_prefix, prefix_cache), id)
                } else {
                    format!("<{}>", v)
                };

                match NamedNode::from_str(&iri) {
                    Ok(t) => Ok(TermPattern::NamedNode(t)),
                    Err(e) => Err(Error::new(ErrorKind::Other, format!("fail convert {:?}{:?} to NamedNode, err={:?}", r.value, r.rtype, e))),
                }
            },
            Str(v, lang) => {
                if lang.is_some() {
                    if let Ok(t) = Literal::new_language_tagged_literal(v.to_string(), lang.to_string()) {
                        Ok(TermPattern::Literal(t))
                    } else {
                        Err(Error::new(ErrorKind::Other, format!("fail convert {:?} to literal, unknown type {:?}", r.value, r.rtype)))
                    }
                } else {
                    Ok(TermPattern::Literal(Literal::new_simple_literal(v.to_string())))
                }
            },
            Int(v) => Ok(TermPattern::Literal(Literal::new_typed_literal(v.to_string(), xsd::INTEGER))),
            Bool(v) => Ok(TermPattern::Literal(Literal::new_typed_literal(v.to_string(), xsd::BOOLEAN))),
            Num(_m, _d) => Ok(TermPattern::Literal(Literal::new_typed_literal(r.get_float().to_string(), xsd::DECIMAL))),
            Datetime(v) => {
                let dt = Utc.timestamp_opt(*v, 0).single().ok_or_else(|| Error::new(ErrorKind::Other, format!("invalid timestamp: {}", v)))?;
                Ok(TermPattern::Literal(Literal::new_typed_literal(format!("{:?}", dt), xsd::DATE_TIME)))
            },
            _ => Err(Error::new(ErrorKind::Other, format!("fail convert {:?} to literal, unknown type {:?}", r.value, r.rtype))),
        };
    }
    Err(Error::new(ErrorKind::Other, "fail convert empty data to literal".to_string()))
}

fn part_copy_termpattern_to_expression(tp: TermPattern) -> io::Result<Expression> {
    match tp {
        TermPattern::NamedNode(v) => Ok(Expression::NamedNode(v)),
        TermPattern::BlankNode(v) => Err(Error::new(ErrorKind::Other, format!("fail convert {:?} from blank node", v))),
        TermPattern::Literal(v) => Ok(Expression::Literal(v)),
        TermPattern::Variable(_) => Err(Error::new(ErrorKind::Other, "fail convert variable to expression".to_string())),
    }
}
