-- Basic constructive theorem definitions for LeanTarget

theorem id_prop {A : Prop} (a : A) : A := a

theorem k_comb {A B : Prop} (a : A) (_ : B) : A := a

theorem modus_ponens_thm {A B : Prop} (a : A) (f : A → B) : B := f a

theorem and_intro_thm {A B : Prop} (a : A) (b : B) : A ∧ B := And.intro a b

theorem trans_impl_thm {A B C : Prop} (f : A → B) (g : B → C) (a : A) : C := g (f a)

theorem curry_thm {A B C : Prop} (f : (A ∧ B) → C) (a : A) (b : B) : C := f (And.intro a b)

theorem and_assoc_thm {A B C : Prop} (h : (A ∧ B) ∧ C) : A ∧ (B ∧ C) :=
  And.intro h.left.left (And.intro h.left.right h.right)

theorem contrapositive_thm {A B : Prop} (f : A → B) (nb : B → False) (a : A) : False :=
  nb (f a)
