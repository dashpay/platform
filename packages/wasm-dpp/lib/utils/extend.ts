export function extend(Derived: any, Base: any) {
    Object.setPrototypeOf(
        Derived.prototype,
        Base.prototype,
    );
}
