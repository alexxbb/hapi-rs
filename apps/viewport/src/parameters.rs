use hapi_rs::node::ParmType;
use hapi_rs::parameter::Parameter;
use hapi_rs::parameter::ParmBaseTrait;
use hapi_rs::Result;


pub struct UiParameter {
    pub parameter: Parameter,
    pub kind: ParmKind,
}

#[derive(Debug)]
pub enum ParmKind {
    Menu { choices: Vec<String>, current: i32 },
    Float { current: f32 },
    Toggle { current: bool },
}

pub fn build_parm_map(parms: Vec<Parameter>) -> Result<Vec<(String, UiParameter)>> {
    let mut parm_list = Vec::new();
    for parm in parms {
        if parm.info().invisible() {
            continue;
        }
        let label = parm.label()?;
        match &parm {
            Parameter::Int(p) => {
                let parmt_type = parm.info().parm_type();
                if parmt_type == ParmType::Toggle {
                    let current = p.get(0)? > 0;
                    parm_list.push((
                        label,
                        UiParameter {
                            parameter: parm,
                            kind: ParmKind::Toggle { current },
                        },
                    ));
                } else {
                    if let Some(menu) = p.menu_items()? {
                        let choices: Vec<String> = menu
                            .into_iter()
                            .map(|choice| choice.label().unwrap())
                            .collect();
                        let current = p.get(0)?;
                        parm_list.push((
                            label,
                            UiParameter {
                                parameter: parm,
                                kind: ParmKind::Menu { choices, current },
                            },
                        ));
                    }
                }
            }
            Parameter::Float(p) => {
                let current = p.get(0)?;
                parm_list.push((
                    label,
                    UiParameter {
                        parameter: parm,
                        kind: ParmKind::Float { current },
                    },
                ));
            }
            _ => {}
        }
    }
    Ok(parm_list)
}
