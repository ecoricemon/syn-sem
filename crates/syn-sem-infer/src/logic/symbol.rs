//! Centralized logic symbol names used by solver terms.

/// Function symbols used to build structured data terms inside predicate arguments.
pub(crate) mod func {
    pub(crate) const ARG: &str = "#arg";
    pub(crate) const ARRAY: &str = "#array";
    pub(crate) const ASSOC_CONST_ARG: &str = "#assoc_const_arg";
    pub(crate) const ASSOC_TYPE_ARG: &str = "#assoc_type_arg";
    pub(crate) const CONST_BOOL: &str = "#const_bool";
    pub(crate) const CONST_FLOAT: &str = "#const_float";
    pub(crate) const CONST_INT: &str = "#const_int";
    pub(crate) const CONST_USIZE: &str = "#const_usize";
    pub(crate) const DEF: &str = "#def";
    pub(crate) const GENERIC_PARAM: &str = "#generic_param";
    pub(crate) const INFER: &str = "#infer";
    pub(crate) const LEN_CONST: &str = "#len_const";
    pub(crate) const LEN_EXPR: &str = "#len_expr";
    pub(crate) const MUT: &str = "#mut";
    pub(crate) const NAME: &str = "#name";
    pub(crate) const PATH: &str = "#path";
    pub(crate) const PRIMITIVE: &str = "#primitive";
    pub(crate) const PRESERVE_GENERICS: &str = "#preserve_generics";
    pub(crate) const REF: &str = "#ref";
    pub(crate) const SLICE: &str = "#slice";
    pub(crate) const TUPLE: &str = "#tuple";
    pub(crate) const VARIABLE_GENERICS: &str = "#variable_generics";
}

/// Atom prefixes used to encode repo ids as zero-argument logic terms.
pub(crate) mod id {
    pub(crate) const DEF: &str = "def";
    pub(crate) const EXPR: &str = "expr";
    pub(crate) const TYPE: &str = "ty";
}

/// Predicate symbols used as logic database relations and queries.
pub(crate) mod pred {
    pub(crate) const EXPLICIT_PROJECTION_OBLIGATION: &str = "#explicit_projection_obligation";
    pub(crate) const IMPL_ASSOC_TYPE: &str = "#impl_assoc_type";
    pub(crate) const IMPL_SELF_MATCH: &str = "#impl_self_match";
    pub(crate) const IMPL_SELF_MATCH_CANDIDATE: &str = "#impl_self_match_candidate";
    pub(crate) const PROJECTION_CANDIDATE: &str = "#projection_candidate";
    pub(crate) const PROJECTION_MATCH: &str = "#projection_match";
    pub(crate) const PROJECTION_NORMALIZES_TO: &str = "#projection_normalizes_to";
    pub(crate) const PROJECTION_OBLIGATION: &str = "#projection_obligation";
    pub(crate) const RESOLVED_TYPE: &str = "#resolved_type";
    pub(crate) const SAME_TYPE: &str = "#same_type";
    pub(crate) const TRAIT_BOUND: &str = "#trait_bound";
    pub(crate) const TYPE_BINDING: &str = "#type_binding";
    pub(crate) const TYPE_CANDIDATE: &str = "#type_candidate";
    pub(crate) const TYPE_EQUAL: &str = "#type_equal";
    pub(crate) const TYPE_SHAPE: &str = "#type_shape";
    pub(crate) const TYPE_SUBSTITUTION: &str = "#type_substitution";
}

/// Variable symbols used in logic rules and open queries.
pub(crate) mod var {
    pub(crate) const ARG: &str = "$Arg";
    pub(crate) const ASSOC: &str = "$Assoc";
    pub(crate) const GENERIC: &str = "$Generic";
    pub(crate) const IMPL_SELF: &str = "$ImplSelf";
    pub(crate) const IMPL_TRAIT: &str = "$ImplTrait";
    pub(crate) const LEFT: &str = "$Left";
    pub(crate) const PROJECTION: &str = "$Projection";
    pub(crate) const RIGHT: &str = "$Right";
    pub(crate) const SELF: &str = "$Self";
    pub(crate) const SUBJECT: &str = "$Subject";
    pub(crate) const SUBSTITUTED: &str = "$Substituted";
    pub(crate) const TRAIT: &str = "$Trait";
    pub(crate) const TYPE: &str = "$Type";
    pub(crate) const VALUE: &str = "$Value";
    pub(crate) const SHAPE: &str = "$Shape";
}
