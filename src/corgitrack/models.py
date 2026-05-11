import enum
import secrets
import uuid
from datetime import datetime, time

from sqlalchemy import BigInteger, Boolean, DateTime, Enum, ForeignKey, Integer, String, Text, Time
from sqlalchemy.orm import Mapped, mapped_column, relationship

from corgitrack.db import Base


class TreatmentKind(str, enum.Enum):
    pill = "pill"
    vaccine = "vaccine"


class DoseStatus(str, enum.Enum):
    planned = "planned"
    reminded = "reminded"
    taken = "taken"
    missed = "missed"
    skipped = "skipped"


def generate_api_key() -> str:
    return secrets.token_urlsafe(32)


class Household(Base):
    __tablename__ = "households"

    id: Mapped[int] = mapped_column(primary_key=True)
    name: Mapped[str] = mapped_column(String(200))
    calendar_token: Mapped[str] = mapped_column(String(64), default=lambda: uuid.uuid4().hex, unique=True)
    created_at: Mapped[datetime] = mapped_column(DateTime(timezone=True), default=datetime.utcnow)

    dogs: Mapped[list["Dog"]] = relationship(back_populates="household")
    members: Mapped[list["FamilyMember"]] = relationship(back_populates="household")


class FamilyMember(Base):
    __tablename__ = "family_members"

    id: Mapped[int] = mapped_column(primary_key=True)
    household_id: Mapped[int] = mapped_column(ForeignKey("households.id", ondelete="CASCADE"))
    display_name: Mapped[str] = mapped_column(String(120))
    telegram_user_id: Mapped[int | None] = mapped_column(BigInteger, unique=True, nullable=True)
    notify: Mapped[bool] = mapped_column(Boolean, default=True)
    created_at: Mapped[datetime] = mapped_column(DateTime(timezone=True), default=datetime.utcnow)

    household: Mapped[Household] = relationship(back_populates="members")


class Dog(Base):
    __tablename__ = "dogs"

    id: Mapped[int] = mapped_column(primary_key=True)
    household_id: Mapped[int] = mapped_column(ForeignKey("households.id", ondelete="CASCADE"))
    name: Mapped[str] = mapped_column(String(120))
    created_at: Mapped[datetime] = mapped_column(DateTime(timezone=True), default=datetime.utcnow)

    household: Mapped[Household] = relationship(back_populates="dogs")
    treatments: Mapped[list["Treatment"]] = relationship(back_populates="dog")


class Treatment(Base):
    __tablename__ = "treatments"

    id: Mapped[int] = mapped_column(primary_key=True)
    dog_id: Mapped[int] = mapped_column(ForeignKey("dogs.id", ondelete="CASCADE"))
    name: Mapped[str] = mapped_column(String(200))
    kind: Mapped[TreatmentKind] = mapped_column(Enum(TreatmentKind))
    dose_label: Mapped[str | None] = mapped_column(String(120), nullable=True)
    cycle_days: Mapped[int] = mapped_column(Integer)
    start_at: Mapped[datetime] = mapped_column(DateTime(timezone=True))
    reminder_time: Mapped[time] = mapped_column(Time, default=time(9, 0))
    instructions: Mapped[str | None] = mapped_column(Text, nullable=True)
    active: Mapped[bool] = mapped_column(Boolean, default=True)
    created_at: Mapped[datetime] = mapped_column(DateTime(timezone=True), default=datetime.utcnow)

    dog: Mapped[Dog] = relationship(back_populates="treatments")
    doses: Mapped[list["Dose"]] = relationship(back_populates="treatment")


class Dose(Base):
    __tablename__ = "doses"

    id: Mapped[int] = mapped_column(primary_key=True)
    treatment_id: Mapped[int] = mapped_column(ForeignKey("treatments.id", ondelete="CASCADE"))
    due_at: Mapped[datetime] = mapped_column(DateTime(timezone=True), index=True)
    status: Mapped[DoseStatus] = mapped_column(Enum(DoseStatus), default=DoseStatus.planned, index=True)
    api_key: Mapped[str] = mapped_column(
        String(64),
        default=generate_api_key,
        unique=True,
        index=True,
    )
    reminded_at: Mapped[datetime | None] = mapped_column(DateTime(timezone=True), nullable=True)
    taken_at: Mapped[datetime | None] = mapped_column(DateTime(timezone=True), nullable=True)
    confirmed_by_member_id: Mapped[int | None] = mapped_column(ForeignKey("family_members.id"), nullable=True)
    note: Mapped[str | None] = mapped_column(Text, nullable=True)
    created_at: Mapped[datetime] = mapped_column(DateTime(timezone=True), default=datetime.utcnow)

    treatment: Mapped[Treatment] = relationship(back_populates="doses")
    confirmed_by: Mapped[FamilyMember | None] = relationship()
