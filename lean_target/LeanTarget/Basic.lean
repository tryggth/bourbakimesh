-- Basic constructive theorem definitions for LeanTarget

theorem id_prop {A : Prop} (a : A) : A := a

theorem k_comb {A B : Prop} (a : A) (_ : B) : A := a

theorem modus_ponens_thm {A B : Prop} (a : A) (f : A → B) : B := f a

theorem and_intro_thm {A B : Prop} (a : A) (b : B) : A ∧ B := And.intro a b

theorem trans_impl_thm {A B C : Prop} (f : A → B) (g : B → C) (a : A) : C := g (f a)
