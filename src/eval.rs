use super::sexp::Node;

#[derive(Debug, Clone)]
pub struct EvalErr(pub Node, pub String);

pub struct Reduction(pub Node, pub u32);

pub type FApply = fn(&Node) -> Result<Reduction, EvalErr>;

pub type FLookup = [Option<FApply>; 256];

impl From<std::io::Error> for EvalErr {
    fn from(err: std::io::Error) -> Self {
        EvalErr(Node::blob("std::io::Error"), err.to_string())
    }
}

impl From<Node> for Reduction {
    fn from(node: Node) -> Self {
        Reduction(node, 1)
    }
}

impl Node {
    pub fn err(&self, msg: &str) -> Result<Reduction, EvalErr> {
        Err(EvalErr(self.clone(), msg.into()))
    }

    pub fn node_err(&self, msg: &str) -> Result<Node, EvalErr> {
        Err(EvalErr(self.clone(), msg.into()))
    }
}

pub type FEval = fn(&Eval, &Node, &Node, u32, u32, u8, u8) -> Result<Reduction, EvalErr>;

pub struct Eval {
    pub eval_f: FEval,
    pub apply_f: FApply0,
    pub f_table: FLookup,
    pub apply3: FApply3,
}

pub type FApply0 = fn(&Eval, &Node, &Node) -> Result<Reduction, EvalErr>;
pub type FApply3 = Box<dyn Fn(&Eval, &Node, &Node) -> Result<Reduction, EvalErr>>;

pub fn default_apply0(
    eval_context: &Eval,
    operator: &Node,
    params: &Node,
) -> Result<Reduction, EvalErr> {
    let op_8: Option<u8> = operator.clone().into();
    if let Some(op_8) = op_8 {
        if let Some(f) = eval_context.f_table[op_8 as usize] {
            return f(&params);
        }
    };
    (eval_context.apply3)(eval_context, operator, params)
}

pub fn default_eval(
    eval_context: &Eval,
    form: &Node,
    env: &Node,
    current_cost: u32,
    max_cost: u32,
    op_quote: u8,
    op_args: u8,
) -> Result<Reduction, EvalErr> {
    match form.as_pair() {
        None => form.err("not a list"),
        Some((left, right)) => {
            if left.is_pair() {
                let r = (eval_context.eval_f)(
                    &eval_context,
                    &left,
                    &env,
                    current_cost,
                    max_cost,
                    op_quote,
                    op_args,
                )?;
                match r {
                    Reduction(result, new_cost) => (eval_context.eval_f)(
                        eval_context,
                        &result.first()?,
                        &result.rest()?,
                        new_cost,
                        max_cost,
                        op_quote,
                        op_args,
                    ),
                }
            } else {
                let as_operator: Option<u8> = left.clone().into();
                if let Some(opcode) = as_operator {
                    if opcode == op_quote {
                        return {
                            let rest = form.rest()?;
                            if rest.nullp() || !rest.rest()?.nullp() {
                                form.err("quote requires exactly 1 parameter")
                            } else {
                                Ok(Reduction(right.first()?, current_cost + 1))
                            }
                        };
                    } else if opcode == op_args {
                        return { Ok(Reduction(env.clone(), current_cost + 1)) };
                    }
                }
                let Reduction(params, new_cost) = eval_params2(
                    &eval_context,
                    &form,
                    &env,
                    current_cost,
                    max_cost,
                    op_quote,
                    op_args,
                )?;
                let Reduction(r, apply_cost) =
                    (eval_context.apply_f)(&eval_context, &left, &params)?;
                Ok(Reduction(r, apply_cost + new_cost))
            }
        }
    }
}

fn eval_params2(
    eval_context: &Eval,
    params: &Node,
    env: &Node,
    current_cost: u32,
    max_cost: u32,
    op_quote: u8,
    op_args: u8,
) -> Result<Reduction, EvalErr> {
    let iter = params.rest()?;
    let mut new_cost = current_cost;
    let mut vec: Vec<Node> = Vec::new();
    for item in iter {
        let r = (eval_context.eval_f)(
            &eval_context,
            &item,
            &env,
            new_cost,
            max_cost,
            op_quote,
            op_args,
        )?;
        vec.push(r.0);
        new_cost += r.1;
        if new_cost >= max_cost {
            return item.err("exceed max cost");
        }
    }
    Ok(Reduction(Node::from_list(vec), new_cost))
}

pub fn default_apply3(
    _eval_context: &Eval,
    operator: &Node,
    _args: &Node,
) -> Result<Reduction, EvalErr> {
    operator.err("unknown operator")
}

pub fn make_default_eval_context(f_lookup: FLookup, apply3: FApply3) -> Eval {
    Eval {
        eval_f: default_eval,
        apply_f: default_apply0,
        f_table: f_lookup,
        apply3: Box::new(apply3),
    }
}

pub fn run_program(
    form: &Node,
    env: &Node,
    current_cost: u32,
    max_cost: u32,
    f_table: &FLookup,
    apply3: FApply3,
    op_quote: u8,
    op_args: u8,
) -> Result<Reduction, EvalErr> {
    let eval_context: Eval = make_default_eval_context(*f_table, apply3);
    (eval_context.eval_f)(
        &eval_context,
        &form,
        &env,
        current_cost,
        max_cost,
        op_quote,
        op_args,
    )
}